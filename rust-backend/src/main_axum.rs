mod cache_manager;
mod config;
mod db;
mod error;
mod middleware;
mod models;
mod retrieval;
mod routes;
mod services;
mod socket;
mod socketio;
mod utils;
mod websocket_chat;

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use crate::config::{Config, MutableConfig};
use crate::db::Database;
use crate::services::sandbox_executor::SandboxExecutorClient;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: MutableConfig,
    pub redis: Option<deadpool_redis::Pool>,
    // Model cache: model_id -> model info (with urlIdx, etc.)
    pub models_cache: Arc<RwLock<std::collections::HashMap<String, serde_json::Value>>>,
    // Socket state for tracking sessions and users (Socket.IO-like functionality)
    pub socket_state: Option<socket::SocketState>,
    // Socket.IO event handler (native Rust implementation)
    pub socketio_handler: Option<Arc<socketio::EventHandler>>,
    // Shared HTTP client for better performance (connection pooling, TLS reuse)
    pub http_client: reqwest::Client,
    // Vector database client for RAG/knowledge base operations
    pub vector_db: Option<Arc<dyn retrieval::VectorDB>>,
    // Embedding provider for generating embeddings
    pub embedding_provider: Option<Arc<dyn retrieval::EmbeddingProvider>>,
    // Sandbox executor client for secure code execution
    pub sandbox_executor_client: Option<Arc<SandboxExecutorClient>>,
    // OAuth session service for managing encrypted OAuth tokens
    pub oauth_session_service: Arc<services::oauth_session::OAuthSessionService>,
    // OAuth manager for coordinating OAuth providers
    pub oauth_manager: Arc<services::oauth_manager::OAuthManager>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    dotenvy::dotenv().ok();

    let log_level = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info".to_string())
        .parse()
        .unwrap_or(Level::INFO);

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Open WebUI Rust Backend with Axum");

    // Load configuration from environment
    let config = Config::from_env()?;
    info!("Configuration loaded from environment");

    // Initialize database (Turso/libSQL)
    let db = Database::new(&config.database_url).await?;
    info!("Database connected (Turso/libSQL)");

    // Run migrations
    db.run_migrations().await?;
    info!("Database migrations completed");

    // Load and merge config from database (PersistentConfig behavior)
    let config = services::ConfigService::load_from_db(&db, config).await?;
    info!("Configuration loaded and merged from database");

    // Initialize Redis if enabled
    let redis = if config.enable_redis {
        let redis_config = deadpool_redis::Config::from_url(&config.redis_url);
        let pool = redis_config.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;
        info!("Redis connected");
        Some(pool)
    } else {
        None
    };

    // Initialize cache manager
    let cache_manager = cache_manager::CacheManager::init(redis.clone());
    cache_manager.start_cleanup_tasks();
    info!("Cache manager initialized and cleanup tasks started");

    // Create shared HTTP client with connection pooling and optimized settings
    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(10) // Reuse connections
        .tcp_nodelay(true) // Disable Nagle's algorithm for real-time streaming
        .timeout(std::time::Duration::from_secs(300)) // 5 min default timeout
        .http2_keep_alive_interval(Some(std::time::Duration::from_secs(5)))
        .http2_keep_alive_while_idle(true)
        .build()?;

    tracing::info!("🌐 HTTP client initialized with connection pooling");

    // Initialize vector database if enabled
    let vector_db_enabled = std::env::var("ENABLE_RAG")
        .unwrap_or_else(|_| "true".to_string())
        .to_lowercase()
        == "true";

    let vector_db = if vector_db_enabled {
        match retrieval::VectorDBFactory::from_env().await {
            Ok(db) => {
                info!("✅ Vector database initialized successfully");
                Some(db)
            }
            Err(e) => {
                warn!("⚠️  Failed to initialize vector database: {}", e);
                None
            }
        }
    } else {
        info!("⚠️  Vector database disabled (ENABLE_RAG=false)");
        None
    };

    // Initialize embedding provider (simplified for now)
    let embedding_provider = None;

    // Initialize sandbox executor client if enabled
    let sandbox_executor_client = if config.enable_code_execution {
        let sandbox_url = config
            .code_execution_sandbox_url
            .clone()
            .unwrap_or_else(|| "http://localhost:8090".to_string());

        info!("🔒 Initializing Sandbox Executor Client");
        info!("   URL: {}", sandbox_url);
        Some(Arc::new(SandboxExecutorClient::new(sandbox_url)))
    } else {
        info!("⚠️  Code execution is disabled");
        None
    };

    // Initialize OAuth session service with encryption
    let oauth_session_service = {
        let encryption_key = &config.oauth_session_token_encryption_key;
        match services::oauth_session::OAuthSessionService::new(db.clone(), encryption_key) {
            Ok(service) => {
                info!("OAuth session service initialized with encryption");
                Arc::new(service)
            }
            Err(e) => {
                warn!("Failed to initialize OAuth session service: {}", e);
                // Create a fallback with a default key
                Arc::new(
                    services::oauth_session::OAuthSessionService::new(
                        db.clone(),
                        &uuid::Uuid::new_v4().to_string(),
                    )
                    .expect("Failed to create OAuth session service"),
                )
            }
        }
    };

    // Initialize OAuth manager
    let oauth_manager = {
        match services::oauth_manager::OAuthManager::new(
            config.clone(),
            oauth_session_service.clone(),
        )
        .await
        {
            Ok(manager) => {
                info!("OAuth manager initialized with providers");
                Arc::new(manager)
            }
            Err(e) => {
                warn!("Failed to initialize OAuth manager: {}", e);
                Arc::new(
                    services::oauth_manager::OAuthManager::new(
                        config.clone(),
                        oauth_session_service.clone(),
                    )
                    .await
                    .expect("Failed to create OAuth manager"),
                )
            }
        }
    };

    let state = Arc::new(AppState {
        db: db.clone(),
        config: Arc::new(RwLock::new(config.clone())),
        redis: redis.clone(),
        models_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        socket_state: None, // TODO: Initialize socket state
        socketio_handler: None, // TODO: Initialize socket.io handler
        http_client,
        vector_db,
        embedding_provider,
        sandbox_executor_client,
        oauth_session_service,
        oauth_manager,
    });

    // Build application with routes and middleware
    let app = Router::new()
        // Health checks
        .route("/health", get(health_check))
        .route("/health/db", get(health_check_db))
        // Config and version
        .route("/api/config", get(get_app_config))
        .route("/api/version", get(get_app_version))
        .route("/api/version/updates", get(get_app_latest_version))
        // Models list endpoint (OpenAI compatible - returns {"data": [...]})
        .route("/api/models", get(get_models))
        .route("/api/models/base", get(get_base_models))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any)
                        .max_age(std::time::Duration::from_secs(3600)),
                ),
        );

    // Start server
    let addr = SocketAddr::from((config.host.parse::<std::net::IpAddr>()?, config.port));
    info!("🚀 Server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Health check endpoints
async fn health_check() -> Json<serde_json::Value> {
    Json(json!({ "status": true }))
}

async fn health_check_db(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, AppError> {
    // Simple database health check
    let conn = state.db.pool().lock().await;
    conn.execute("SELECT 1", ()).await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(json!({ "status": true })))
}

async fn get_app_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let config = state.config.read().unwrap();

    // Get actual user count from database
    let user_service = services::user::UserService::new(&state.db);
    let user_count = user_service.get_user_count().await.unwrap_or(0);
    let onboarding = user_count == 0;

    let mut response = json!({
        "status": true,
        "name": config.webui_name,
        "version": env!("CARGO_PKG_VERSION"),
        "default_locale": "en-US",
        "features": {
            "auth": config.webui_auth,
            "auth_trusted_header": false,
            "enable_signup": config.enable_signup,
            "enable_login_form": config.enable_login_form,
            "enable_api_key": config.enable_api_key,
            "enable_ldap": false,
            "enable_websocket": config.enable_websocket_support,
            "enable_version_update_check": config.enable_version_update_check,
            "enable_signup_password_confirmation": false,
        },
        "oauth": {
            "providers": {}
        }
    });

    if onboarding {
        response["onboarding"] = json!(true);
    }

    Json(response)
}

async fn get_app_version() -> Json<serde_json::Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn get_app_latest_version() -> Json<serde_json::Value> {
    let current_version = env!("CARGO_PKG_VERSION");
    Json(json!({
        "current": current_version,
        "latest": current_version,
    }))
}

async fn get_models(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let config = state.config.read().unwrap().clone();
    let model_service = crate::services::models::ModelService::new(config.clone());

    match model_service.get_all_models(&state.db).await {
        Ok(models) => Json(json!({
            "data": models
        })),
        Err(e) => {
            tracing::error!("Failed to fetch models: {}", e);
            Json(json!({
                "data": []
            }))
        }
    }
}

async fn get_base_models(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let config = state.config.read().unwrap().clone();
    let model_service = crate::services::models::ModelService::new(config);

    match model_service.get_all_base_models(&state.db).await {
        Ok(models) => Json(json!({
            "data": models
        })),
        Err(e) => {
            tracing::error!("Failed to fetch base models: {}", e);
            Json(json!({
                "data": []
            }))
        }
    }
}

// Error handling
#[derive(Debug)]
enum AppError {
    Database(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}
