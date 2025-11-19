use axum::{
    extract::State,
    response::Json,
    routing::post,
    Router,
};
use serde_json::json;
use validator::Validate;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::models::{SessionResponse, SigninRequest, SignupRequest};
use crate::services::{AuthService, UserService};
use crate::utils::auth::create_jwt;
use crate::AppState;

pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/signin", post(signin))
        .route("/signup", post(signup))
}

async fn signin(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SigninRequest>,
) -> AppResult<Json<SessionResponse>> {
    // Validate input
    payload.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let auth_service = AuthService::new(&state.db);
    let user_service = UserService::new(&state.db);

    // Authenticate user
    let user_id = auth_service
        .authenticate(&payload.email, &payload.password)
        .await?
        .ok_or_else(|| AppError::InvalidCredentials)?;

    // Get user details
    let user = user_service
        .get_user_by_id(&user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Create JWT token
    let config = state.config.read().unwrap();
    let token = create_jwt(&user.id, &user.role, &config.webui_secret_key, config.session_expiry_seconds)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(Json(SessionResponse {
        token: token.clone(),
        token_type: "Bearer".to_string(),
        expires_at: None,
        id: user.id,
        email: user.email,
        name: user.name,
        role: user.role,
        profile_image_url: user.profile_image_url,
        permissions: json!({}),
    }))
}

async fn signup(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SignupRequest>,
) -> AppResult<Json<SessionResponse>> {
    // Validate input
    payload.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let config = state.config.read().unwrap();

    // Check if signup is enabled
    if !config.enable_signup {
        return Err(AppError::Forbidden("Signup is disabled".to_string()));
    }

    let auth_service = AuthService::new(&state.db);
    let user_service = UserService::new(&state.db);

    // Check if user already exists
    if user_service.get_user_by_email(&payload.email).await?.is_some() {
        return Err(AppError::UserAlreadyExists);
    }

    // Create user ID
    let user_id = uuid::Uuid::new_v4().to_string();

    // Determine role (first user is admin)
    let user_count = user_service.count_users().await?;
    let role = if user_count == 0 { "admin" } else { "user" };

    // Create user
    let profile_image_url = format!("/user.png");
    user_service
        .create_user(&user_id, &payload.name, &payload.email, role, &profile_image_url)
        .await?;

    // Create auth
    auth_service
        .create_auth(&user_id, &payload.email, &payload.password)
        .await?;

    // Get created user
    let user = user_service
        .get_user_by_id(&user_id)
        .await?
        .ok_or_else(|| AppError::InternalServerError("Failed to create user".to_string()))?;

    // Create JWT token
    let token = create_jwt(&user.id, &user.role, &config.webui_secret_key, config.session_expiry_seconds)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(Json(SessionResponse {
        token: token.clone(),
        token_type: "Bearer".to_string(),
        expires_at: None,
        id: user.id,
        email: user.email,
        name: user.name,
        role: user.role,
        profile_image_url: user.profile_image_url,
        permissions: json!({}),
    }))
}
