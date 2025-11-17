/// OAuth Routes
/// Handles OAuth login and callback endpoints
use crate::error::{AppError, AppResult};
use crate::services::oauth_provider::OAuthUserInfo;
use crate::utils::auth::create_jwt;
use crate::AppState;
use actix_web::{cookie::Cookie, web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

/// Query parameters for OAuth login endpoint
#[derive(Debug, Deserialize)]
pub struct OAuthLoginQuery {
    pub redirect_url: Option<String>,
}

/// Query parameters for OAuth callback endpoint
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

/// Response for OAuth login (redirect to provider)
#[derive(Debug, Serialize)]
pub struct OAuthLoginResponse {
    pub authorization_url: String,
}

/// Register OAuth routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/oauth")
            .route("/{provider}/login", web::get().to(oauth_login))
            .route("/{provider}/callback", web::get().to(oauth_callback))
            // Legacy endpoint support
            .route("/{provider}/login/callback", web::get().to(oauth_callback)),
    );
}

/// OAuth login endpoint - initiates OAuth flow
async fn oauth_login(
    state: web::Data<AppState>,
    provider: web::Path<String>,
    query: web::Query<OAuthLoginQuery>,
) -> AppResult<HttpResponse> {
    let provider_name = provider.into_inner();

    debug!("OAuth login request for provider: {}", provider_name);

    // Check if provider is configured
    if !state
        .oauth_manager
        .is_provider_configured(&provider_name)
        .await
    {
        return Err(AppError::NotFound(format!(
            "OAuth provider '{}' is not configured",
            provider_name
        )));
    }

    // Check if OAuth signup is enabled
    let config = state.config.read().unwrap();
    if !config.enable_oauth_signup {
        return Err(AppError::Forbidden(
            "OAuth authentication is disabled".to_string(),
        ));
    }
    drop(config);

    // Initiate OAuth flow
    let redirect_path = query.redirect_url.clone();
    let authorization_url = state
        .oauth_manager
        .initiate_login(&provider_name, redirect_path)
        .await?;

    info!("OAuth login initiated for provider: {}", provider_name);

    // Return redirect response
    Ok(HttpResponse::Found()
        .append_header(("Location", authorization_url))
        .finish())
}

/// OAuth callback endpoint - handles provider redirect
async fn oauth_callback(
    state: web::Data<AppState>,
    provider: web::Path<String>,
    query: web::Query<OAuthCallbackQuery>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let provider_name = provider.into_inner();

    debug!(
        "OAuth callback for provider: {} with state: {}",
        provider_name, query.state
    );

    // Handle OAuth callback
    let (returned_provider, token_response, user_info) = state
        .oauth_manager
        .handle_callback(&query.code, &query.state)
        .await
        .map_err(|e| {
            error!("OAuth callback failed: {}", e);
            e
        })?;

    // Verify provider matches
    if returned_provider != provider_name {
        return Err(AppError::Auth(format!(
            "Provider mismatch: expected {}, got {}",
            provider_name, returned_provider
        )));
    }

    info!(
        "OAuth callback successful for provider: {} (user: {})",
        provider_name, user_info.sub
    );

    // Find or create user
    let user = find_or_create_user(&state, &provider_name, &user_info).await?;

    // Sync user groups from OAuth (if enabled)
    if let Err(e) = sync_user_groups_from_oauth(&state, &user.id, &user_info).await {
        tracing::warn!("Failed to sync user groups from OAuth: {}", e);
        // Non-fatal error, continue with authentication
    }

    // Store id_token for cookie (if available)
    let token_response_for_cookie = token_response.id_token.clone();

    // Create OAuth session
    state
        .oauth_manager
        .create_session(&user.id, &provider_name, token_response)
        .await?;

    // Generate JWT token
    let config = state.config.read().unwrap();
    let jwt_token = create_jwt(&user.id, &config.webui_secret_key, &config.jwt_expires_in)?;
    drop(config);

    // Create cookies
    let mut response = HttpResponse::Found();

    // Set auth cookie
    let auth_cookie = Cookie::build("token", jwt_token.clone())
        .path("/")
        .http_only(true)
        .secure(req.connection_info().scheme() == "https")
        .same_site(actix_web::cookie::SameSite::Lax)
        .finish();

    response.cookie(auth_cookie);

    // Set ID token cookie if enabled and available
    let config = state.config.read().unwrap();
    if config.enable_oauth_id_token_cookie {
        // Set ID token cookie if available in token response
        if let Some(id_token) = token_response_for_cookie {
            let id_token_cookie = Cookie::build("id_token", id_token)
                .path("/")
                .http_only(true)
                .secure(req.connection_info().scheme() == "https")
                .same_site(actix_web::cookie::SameSite::Lax)
                .finish();

            response.cookie(id_token_cookie);
            debug!("Set ID token cookie");
        }
    }

    // Determine redirect URL
    let redirect_url = format!("{}/", config.frontend_base_url);
    drop(config);

    response.append_header(("Location", redirect_url));

    info!(
        "OAuth login completed for user: {} ({})",
        user.name, user.email
    );

    Ok(response.finish())
}

/// Find or create user from OAuth information
async fn find_or_create_user(
    state: &web::Data<AppState>,
    provider: &str,
    user_info: &OAuthUserInfo,
) -> AppResult<crate::models::user::User> {
    let config = state.config.read().unwrap();

    // Build OAuth sub identifier (provider@user_id)
    let oauth_sub = format!("{}@{}", provider, user_info.sub);

    // Try to find user by oauth_sub
    let user = sqlx::query_as::<_, crate::models::user::User>(
        "SELECT * FROM \"user\" WHERE oauth_sub = $1",
    )
    .bind(&oauth_sub)
    .fetch_optional(state.db.pool())
    .await?;

    if let Some(user) = user {
        debug!("Found existing user by oauth_sub: {}", user.id);
        return Ok(user);
    }

    // Extract email from user info
    let email = user_info
        .email
        .as_ref()
        .ok_or_else(|| AppError::Auth("Email not provided by OAuth provider".to_string()))?;

    // Validate email domain
    if !state.oauth_manager.is_email_domain_allowed(email) {
        return Err(AppError::Forbidden(format!(
            "Email domain not allowed: {}",
            email
        )));
    }

    // Check if we should merge accounts by email
    if config.oauth_merge_accounts_by_email {
        let user = sqlx::query_as::<_, crate::models::user::User>(
            "SELECT * FROM \"user\" WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(state.db.pool())
        .await?;

        if let Some(mut user) = user {
            // Link OAuth account to existing user
            debug!("Merging OAuth account with existing user: {}", user.id);

            // Update user with oauth_sub
            sqlx::query("UPDATE \"user\" SET oauth_sub = $1, updated_at = $2 WHERE id = $3")
                .bind(&oauth_sub)
                .bind(chrono::Utc::now().timestamp())
                .bind(&user.id)
                .execute(state.db.pool())
                .await?;

            user.oauth_sub = Some(oauth_sub);

            info!("Linked OAuth account to existing user: {}", user.id);
            return Ok(user);
        }
    }

    // Check if OAuth signup is enabled
    if !config.enable_oauth_signup {
        return Err(AppError::Forbidden(
            "OAuth signup is disabled. Contact administrator.".to_string(),
        ));
    }

    // Check if this is the first user
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM \"user\"")
        .fetch_one(state.db.pool())
        .await?;

    let is_first_user = user_count == 0;

    // Determine user role
    let role = state
        .oauth_manager
        .determine_user_role(user_info, is_first_user);

    // If role is "pending", user is not allowed
    if role == "pending" {
        return Err(AppError::Forbidden(
            "Your account does not have the required roles to access this application.".to_string(),
        ));
    }

    // Extract username
    let username = extract_username(user_info, email);

    // Download and encode profile picture if available
    let profile_image_url = if let Some(picture_url) = &user_info.picture {
        match download_and_encode_profile_picture(state, picture_url).await {
            Ok(data_url) => data_url,
            Err(e) => {
                tracing::warn!("Failed to download profile picture: {}", e);
                String::new() // Empty string on failure
            }
        }
    } else {
        String::new()
    };

    let user_id = uuid::Uuid::new_v4().to_string();
    let user_name = user_info.name.clone().unwrap_or_else(|| username.clone());
    let current_time = chrono::Utc::now().timestamp();

    drop(config);

    // Create new user
    let user = sqlx::query_as::<_, crate::models::user::User>(
        r#"
        INSERT INTO "user" (
            id, name, email, role, profile_image_url, oauth_sub, 
            last_active_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(&user_id)
    .bind(&user_name)
    .bind(email)
    .bind(&role)
    .bind(&profile_image_url)
    .bind(&oauth_sub)
    .bind(current_time)
    .bind(current_time)
    .bind(current_time)
    .fetch_one(state.db.pool())
    .await?;

    info!(
        "Created new user from OAuth: {} ({}) with role: {}",
        user.name, user.email, user.role
    );

    // Send webhook notification
    send_oauth_user_signup_webhook(state, &user).await;

    Ok(user)
}

/// Extract username from OAuth user info
fn extract_username(user_info: &OAuthUserInfo, email: &str) -> String {
    // Try to get from name
    if let Some(name) = &user_info.name {
        return name.clone();
    }

    // Try to get from given_name
    if let Some(given_name) = &user_info.given_name {
        return given_name.clone();
    }

    // Extract from email (before @)
    email.split('@').next().unwrap_or(email).to_string()
}

/// Send webhook notification for OAuth user signup
async fn send_oauth_user_signup_webhook(
    state: &web::Data<AppState>,
    user: &crate::models::user::User,
) {
    let config = state.config.read().unwrap();
    let webhook_url = config.webhook_url.clone();
    drop(config);

    if webhook_url.is_none() || webhook_url.as_ref().unwrap().is_empty() {
        return;
    }

    let url = webhook_url.unwrap();

    let payload = crate::utils::webhook::WebhookPayload::new(
        "oauth.user.signup",
        serde_json::json!({
            "user_id": user.id,
            "name": user.name,
            "email": user.email,
            "role": user.role,
            "oauth_sub": user.oauth_sub,
        }),
    );

    if let Err(e) = crate::utils::webhook::post_webhook(&url, payload).await {
        tracing::warn!("Failed to send OAuth user signup webhook: {}", e);
    } else {
        debug!("Sent OAuth user signup webhook for user: {}", user.id);
    }
}

/// Download and encode profile picture to base64 data URL
async fn download_and_encode_profile_picture(
    state: &web::Data<AppState>,
    picture_url: &str,
) -> AppResult<String> {
    // Download the image
    let response = state
        .http_client
        .get(picture_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| {
            AppError::ExternalServiceError(format!("Failed to download profile picture: {}", e))
        })?;

    if !response.status().is_success() {
        return Err(AppError::ExternalServiceError(format!(
            "Failed to download profile picture: HTTP {}",
            response.status()
        )));
    }

    // Get content type
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();

    // Read image bytes
    let bytes = response.bytes().await.map_err(|e| {
        AppError::ExternalServiceError(format!("Failed to read profile picture: {}", e))
    })?;

    // Limit image size to 5MB
    if bytes.len() > 5 * 1024 * 1024 {
        return Err(AppError::BadRequest(
            "Profile picture is too large (max 5MB)".to_string(),
        ));
    }

    // Encode to base64
    use base64::Engine;
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&bytes);

    // Create data URL
    Ok(format!("data:{};base64,{}", content_type, base64_data))
}

/// Sync user groups from OAuth claims
async fn sync_user_groups_from_oauth(
    state: &web::Data<AppState>,
    user_id: &str,
    user_info: &OAuthUserInfo,
) -> AppResult<()> {
    let config = state.config.read().unwrap();

    // Check if group management is enabled
    if !config.enable_oauth_group_management {
        return Ok(());
    }

    // Extract groups from OAuth claims
    let oauth_groups = state.oauth_manager.extract_groups(user_info);

    if oauth_groups.is_empty() {
        debug!("No groups found in OAuth claims for user {}", user_id);
        return Ok(());
    }

    info!(
        "Syncing {} OAuth groups for user {}",
        oauth_groups.len(),
        user_id
    );

    // Filter out blocked groups
    let blocked_groups = &config.oauth_blocked_groups;
    let allowed_groups: Vec<String> = oauth_groups
        .into_iter()
        .filter(|g| !blocked_groups.contains(g))
        .collect();

    if allowed_groups.is_empty() {
        debug!("All OAuth groups are blocked for user {}", user_id);
        return Ok(());
    }

    drop(config);

    // Get or create groups
    for group_name in allowed_groups {
        // Check if group already exists by name
        let existing_group =
            sqlx::query_scalar::<_, String>(r#"SELECT id FROM "group" WHERE name = $1"#)
                .bind(&group_name)
                .fetch_optional(state.db.pool())
                .await?;

        let group_id = if let Some(group_id) = existing_group {
            group_id
        } else {
            // Create group if group creation is enabled
            let config = state.config.read().unwrap();
            if !config.enable_oauth_group_creation {
                debug!("Skipping group creation for '{}' (disabled)", group_name);
                drop(config);
                continue;
            }
            drop(config);

            let new_group_id = uuid::Uuid::new_v4().to_string();
            let current_time = chrono::Utc::now().timestamp();

            sqlx::query(
                r#"
                INSERT INTO "group" (id, user_id, name, description, meta, permissions, user_ids, created_at, updated_at)
                VALUES ($1, $2, $3, $4, NULL, NULL, '[]'::jsonb, $5, $6)
                ON CONFLICT (name) DO NOTHING
                "#
            )
            .bind(&new_group_id)
            .bind(user_id) // Creator is the OAuth user
            .bind(&group_name)
            .bind(format!("Auto-created from OAuth: {}", group_name))
            .bind(current_time)
            .bind(current_time)
            .execute(state.db.pool())
            .await?;

            info!("Created new group from OAuth: {}", group_name);
            new_group_id
        };

        // Add user to group if not already a member
        let is_member = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM "group" 
                WHERE id = $1 
                AND user_ids::jsonb @> $2::jsonb
            )
            "#,
        )
        .bind(&group_id)
        .bind(serde_json::json!([user_id]).to_string())
        .fetch_one(state.db.pool())
        .await?;

        if !is_member {
            // Add user to group's user_ids array
            sqlx::query(
                r#"
                UPDATE "group"
                SET user_ids = COALESCE(user_ids, '[]'::jsonb) || $2::jsonb,
                    updated_at = $3
                WHERE id = $1
                "#,
            )
            .bind(&group_id)
            .bind(serde_json::json!([user_id]).to_string())
            .bind(chrono::Utc::now().timestamp())
            .execute(state.db.pool())
            .await?;

            info!("Added user {} to group {}", user_id, group_name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_username() {
        let user_info = OAuthUserInfo {
            sub: "123".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: Some(true),
            name: Some("Test User".to_string()),
            given_name: None,
            family_name: None,
            picture: None,
            locale: None,
            extra: Default::default(),
        };

        assert_eq!(
            extract_username(&user_info, "test@example.com"),
            "Test User"
        );

        let user_info_no_name = OAuthUserInfo {
            sub: "123".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: Some(true),
            name: None,
            given_name: Some("Test".to_string()),
            family_name: None,
            picture: None,
            locale: None,
            extra: Default::default(),
        };

        assert_eq!(
            extract_username(&user_info_no_name, "test@example.com"),
            "Test"
        );

        let user_info_email_only = OAuthUserInfo {
            sub: "123".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: Some(true),
            name: None,
            given_name: None,
            family_name: None,
            picture: None,
            locale: None,
            extra: Default::default(),
        };

        assert_eq!(
            extract_username(&user_info_email_only, "test@example.com"),
            "test"
        );
    }
}
