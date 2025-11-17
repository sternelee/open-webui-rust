# Migration Guide: Actix-web to Axum and PostgreSQL to Turso

This document outlines the migration strategy for converting the Open WebUI Rust backend from actix-web to axum and from PostgreSQL to Turso (libSQL).

## Overview

The migration involves two major components:
1. **Web Framework**: actix-web → axum
2. **Database**: PostgreSQL (via sqlx) → Turso/libSQL

## Current Status

### Completed
- ✅ Updated Cargo.toml dependencies (axum, tower, tower-http, libsql)
- ✅ Created new database module (src/db.rs) using libSQL
- ✅ Created SQLite migration schema (migrations/sqlite/001_initial.sql)
- ✅ Created example axum main.rs (src/main_axum.rs)

### Remaining Work

#### 1. Database Layer Migration (High Priority)

**Files to update:**
- [ ] All service modules in `src/services/`
  - `auth.rs`, `chat.rs`, `user.rs`, `models.rs`, etc.
  - Replace sqlx query macros with libSQL API
  - Update parameter binding from `$1, $2` (PostgreSQL) to `?` (SQLite)
  
**Pattern to follow:**
```rust
// Old (sqlx):
sqlx::query_as::<_, User>("SELECT * FROM user WHERE id = $1")
    .bind(user_id)
    .fetch_one(&pool)
    .await?

// New (libSQL):
let conn = db.pool().lock().await;
let mut rows = conn.query("SELECT * FROM user WHERE id = ?", [user_id]).await?;
if let Some(row) = rows.next().await? {
    // Manual row parsing
}
```

**Migration files:**
- [ ] Convert remaining PostgreSQL migrations (002-010) to SQLite
- Key differences:
  - `JSONB` → `TEXT` (store as JSON string)
  - `BOOLEAN` → `INTEGER` (0/1)
  - `VARCHAR(n)` → `TEXT`
  - `BIGINT` → `INTEGER`
  - `DATE` → `TEXT`
  - Remove PL/pgSQL blocks (SQLite doesn't support)

#### 2. Web Framework Migration (High Priority)

**Core changes needed:**

**a. Main application setup (src/main.rs)**
- [ ] Replace `#[actix_web::main]` with `#[tokio::main]`
- [ ] Replace `HttpServer::new()` with axum `Router` and `axum::serve()`
- [ ] Convert middleware from actix to tower/axum
- [ ] Update CORS from `actix-cors` to `tower-http::cors`

**b. Route handlers (src/routes/*.rs)**
All route modules need conversion:
- [ ] auth.rs
- [ ] chats.rs  
- [ ] openai.rs
- [ ] models.rs
- [ ] files.rs
- [ ] knowledge.rs
- [ ] prompts.rs
- [ ] tools.rs
- [ ] functions.rs
- [ ] folders.rs
- [ ] groups.rs
- [ ] users.rs
- [ ] notes.rs
- [ ] channels.rs
- [ ] memories.rs
- [ ] audio.rs
- [ ] images.rs
- [ ] tasks.rs
- [ ] webhooks.rs
- [ ] retrieval.rs
- [ ] oauth.rs
- [ ] configs.rs

**Handler pattern:**
```rust
// Old (actix-web):
async fn handler(
    state: web::Data<AppState>,
    payload: web::Json<Request>,
) -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().json(response))
}

// New (axum):
async fn handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Request>,
) -> Result<Json<Response>, AppError> {
    Ok(Json(response))
}
```

**c. Middleware (src/middleware/)**
- [ ] Convert `AuthMiddleware` from actix to tower
- [ ] Update `SecurityHeaders` middleware

**Pattern:**
```rust
// Old (actix):
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};

// New (axum/tower):
use tower::{Layer, Service};
use axum::middleware::{self, Next};
```

**d. WebSocket handling**
- [ ] Replace `actix-ws` with `axum::extract::ws`
- [ ] Update Socket.IO implementation in `src/socketio/` 
- [ ] Update WebSocket chat handler in `src/websocket_chat.rs`

**Pattern:**
```rust
// Old (actix-ws):
actix_ws::handle(&req, stream)

// New (axum-ws):
use axum::extract::ws::{WebSocketUpgrade, WebSocket};
async fn websocket_handler(
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket: WebSocket| async {
        // Handle websocket
    })
}
```

**e. Error handling**
- [ ] Update `src/error.rs` to implement `IntoResponse` for axum
- [ ] Remove actix-web specific error types

**Pattern:**
```rust
// Old:
impl actix_web::ResponseError for AppError {}

// New:
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": self.to_string()}))).into_response()
    }
}
```

#### 3. Extractors and State Management

**State access:**
```rust
// Old (actix-web):
async fn handler(state: web::Data<AppState>) { }

// New (axum):
async fn handler(State(state): State<Arc<AppState>>) { }
```

**Request extractors:**
```rust
// Old:
web::Json(payload)
web::Path(id)
web::Query(params)

// New:
Json(payload): Json<T>
Path(id): Path<String>
Query(params): Query<HashMap<String, String>>
```

**Authentication:**
```rust
// Old (custom extractor):
auth_user: middleware::AuthUser

// New (axum middleware or extractor):
Extension(user): Extension<User>
// Or create custom extractor implementing FromRequestParts
```

#### 4. Static File Serving

```rust
// Old:
use actix_files::Files;
.service(Files::new("/static", "./static"))

// New:
use tower_http::services::ServeDir;
.nest_service("/static", ServeDir::new("./static"))
```

#### 5. Streaming Responses

For chat completions streaming:
```rust
// Old (actix-web):
HttpResponse::Ok()
    .content_type("text/event-stream")
    .streaming(stream)

// New (axum):
use axum::response::Sse;
use futures::stream::Stream;
Sse::new(stream).keep_alive(KeepAlive::default())
```

#### 6. Multipart Form Data

```rust
// Old:
use actix_multipart::Multipart;

// New:
use axum::extract::Multipart;
```

## Testing Strategy

1. **Unit tests**: Update all unit tests to use axum test helpers
2. **Integration tests**: Test each endpoint after migration
3. **Database tests**: Verify SQLite migrations work correctly
4. **Performance tests**: Compare with actix-web version

## Migration Steps (Recommended Order)

1. ✅ Update dependencies
2. ✅ Create new database module
3. ✅ Convert database migrations
4. [ ] Update error handling
5. [ ] Migrate core services (user, auth, config)
6. [ ] Create new main.rs with basic routes
7. [ ] Migrate authentication middleware
8. [ ] Migrate remaining route handlers one by one
9. [ ] Update WebSocket handlers
10. [ ] Update Socket.IO implementation
11. [ ] Test and fix issues
12. [ ] Performance tuning
13. [ ] Documentation update

## Key Dependencies

```toml
[dependencies]
# Web framework
axum = { version = "0.7", features = ["ws", "multipart", "macros"] }
axum-extra = { version = "0.9", features = ["cookie", "typed-header"] }
tower = { version = "0.5", features = ["full"] }
tower-http = { version = "0.6", features = ["fs", "cors", "compression-full", "trace"] }
hyper = { version = "1.5", features = ["full"] }

# Database  
libsql = "0.6"

# Keep existing
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
# ... other dependencies
```

## Notes

- **SQLite limitations**: No native JSONB type, use TEXT with JSON strings
- **Boolean values**: SQLite uses INTEGER (0/1) instead of BOOLEAN
- **Concurrent writes**: SQLite/Turso may have different concurrency characteristics
- **Connection pooling**: libSQL handles connections differently than sqlx
- **Turso-specific**: When using remote Turso, set `TURSO_AUTH_TOKEN` environment variable

## Benefits After Migration

- Modern, ergonomic API with axum
- Better type safety with extractors
- Simplified middleware composition with tower
- Potential performance improvements
- Distributed database capabilities with Turso
- Lower infrastructure costs (Turso has generous free tier)

## References

- [Axum Documentation](https://docs.rs/axum/)
- [Tower Documentation](https://docs.rs/tower/)
- [libSQL Documentation](https://docs.turso.tech/libsql)
- [Turso Documentation](https://docs.turso.tech/)
