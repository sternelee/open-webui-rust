use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::models::UserResponse;
use crate::services::UserService;
use crate::AppState;

pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_users))
        .route("/count", get(count_users))
        .route("/:id", get(get_user_by_id))
        .route("/:id", delete(delete_user))
        .route("/:id/role", post(update_user_role))
}

#[derive(Deserialize)]
struct ListUsersQuery {
    skip: Option<i64>,
    limit: Option<i64>,
    page: Option<i64>,
}

#[derive(Deserialize)]
struct UpdateRoleRequest {
    role: String,
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListUsersQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let user_service = UserService::new(&state.db);
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(30).min(100);
    let skip = query.skip.unwrap_or((page - 1) * limit);

    let users = user_service.list_users(skip, limit).await?;
    let total = user_service.count_users().await?;

    Ok(Json(json!({
        "users": users.into_iter().map(UserResponse::from).collect::<Vec<_>>(),
        "total": total,
        "page": page,
        "limit": limit
    })))
}

async fn count_users(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    let user_service = UserService::new(&state.db);
    let count = user_service.count_users().await?;

    Ok(Json(json!({
        "count": count
    })))
}

async fn get_user_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<UserResponse>> {
    let user_service = UserService::new(&state.db);
    let user = user_service
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(UserResponse::from(user)))
}

async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let user_service = UserService::new(&state.db);
    user_service.delete_user(&id).await?;

    Ok(Json(json!({
        "status": "success",
        "message": "User deleted"
    })))
}

async fn update_user_role(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateRoleRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let user_service = UserService::new(&state.db);
    user_service.update_user_role(&id, &payload.role).await?;

    Ok(Json(json!({
        "status": "success",
        "message": "Role updated"
    })))
}
