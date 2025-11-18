mod config;
mod db;
mod error;

use axum::{
    extract::State,
    response::Json,
    routing::get,
    Router,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::config::{Config, MutableConfig};
use crate::db::Database;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: MutableConfig,
    pub http_client: reqwest::Client,
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

    info!("🚀 Starting Open WebUI Rust Backend with Axum");

    // Load configuration from environment
    let config = Config::from_env()?;
    info!("✅ Configuration loaded from environment");

    // Initialize database (Turso/libSQL)
    let db = Database::new(&config.database_url).await?;
    info!("✅ Database connected (Turso/libSQL)");

    // Run migrations
    db.run_migrations().await?;
    info!("✅ Database migrations completed");

    // Create shared HTTP client
    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .tcp_nodelay(true)
        .timeout(std::time::Duration::from_secs(300))
        .http2_keep_alive_interval(Some(std::time::Duration::from_secs(5)))
        .http2_keep_alive_while_idle(true)
        .build()?;

    info!("✅ HTTP client initialized");

    let state = Arc::new(AppState {
        db: db.clone(),
        config: Arc::new(RwLock::new(config.clone())),
        http_client,
    });

    // Build application with routes and middleware
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/health/db", get(health_check_db))
        .route("/api/config", get(get_app_config))
        .route("/api/version", get(get_app_version))
        .route("/api/version/updates", get(get_app_latest_version))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(CorsLayer::permissive().max_age(std::time::Duration::from_secs(3600))),
        );

    // Start server
    let addr = SocketAddr::from((config.host.parse::<std::net::IpAddr>()?, config.port));
    info!("🚀 Axum server running at http://{}", addr);
    info!("📝 Migration Status:");
    info!("   ✅ Web Framework: Axum (from actix-web)");
    info!("   ✅ Database: Turso/libSQL (from PostgreSQL)");
    info!("   ✅ Middleware: Tower (from actix)");
    info!("   ✅ Error Handling: IntoResponse");
    info!("   ⚠️  Service APIs: In progress (83 files remaining)");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({ "status": true }))
}

async fn health_check_db(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, error::AppError> {
    let pool = state.db.pool();
    let conn = pool.lock().await;
    conn.execute("SELECT 1", ())
        .await
        .map_err(|e| error::AppError::Database(e.to_string()))?;

    Ok(Json(json!({ "status": true })))
}

async fn get_app_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let config = state.config.read().unwrap();

    Json(json!({
        "status": true,
        "name": config.webui_name,
        "version": env!("CARGO_PKG_VERSION"),
        "default_locale": "en-US",
        "features": {
            "auth": config.webui_auth,
            "enable_signup": config.enable_signup,
            "enable_login_form": config.enable_login_form,
            "enable_api_key": config.enable_api_key,
            "enable_websocket": false,
        },
        "migration": {
            "status": "in_progress",
            "framework": "axum",
            "database": "turso/libsql",
            "completed": ["dependencies", "error_handling", "database_layer", "main_framework"],
            "remaining": ["services", "routes", "middleware", "websockets"]
        }
    }))
}

async fn get_app_version() -> Json<serde_json::Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "framework": "axum",
        "database": "turso/libsql"
    }))
}

async fn get_app_latest_version() -> Json<serde_json::Value> {
    let current_version = env!("CARGO_PKG_VERSION");
    Json(json!({
        "current": current_version,
        "latest": current_version,
    }))
}
