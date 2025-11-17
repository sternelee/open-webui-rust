/// OAuth Manager
/// Coordinates multiple OAuth providers and manages the OAuth flow
use super::oauth_provider::{
    create_feishu_provider, create_github_provider, create_google_provider,
    create_microsoft_provider, create_oidc_provider, BaseOAuthProvider, OAuthProvider,
    OAuthTokenResponse, OAuthUserInfo, PKCEData,
};
use super::oauth_session::OAuthSessionService;
use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::oauth_session::OAuthTokenData;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// OAuth state data stored temporarily during OAuth flow
#[derive(Debug, Clone)]
pub struct OAuthState {
    pub provider: String,
    pub pkce: Option<PKCEData>,
    pub redirect_path: Option<String>,
    pub created_at: i64,
}

/// OAuth Manager - coordinates all OAuth providers
pub struct OAuthManager {
    providers: Arc<RwLock<HashMap<String, Arc<dyn OAuthProvider>>>>,
    states: Arc<RwLock<HashMap<String, OAuthState>>>,
    session_service: Arc<OAuthSessionService>,
    config: Config,
}

impl OAuthManager {
    /// Create a new OAuth manager
    pub async fn new(config: Config, session_service: Arc<OAuthSessionService>) -> AppResult<Self> {
        let manager = Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            session_service,
            config: config.clone(),
        };

        // Initialize providers
        manager.initialize_providers(&config).await?;

        Ok(manager)
    }

    /// Initialize all configured OAuth providers
    async fn initialize_providers(&self, config: &Config) -> AppResult<()> {
        let mut providers = self.providers.write().await;
        let mut count = 0;

        // Google
        if let Some(provider) = create_google_provider(config).await? {
            providers.insert("google".to_string(), Arc::new(provider));
            count += 1;
        }

        // Microsoft
        if let Some(provider) = create_microsoft_provider(config).await? {
            providers.insert("microsoft".to_string(), Arc::new(provider));
            count += 1;
        }

        // GitHub
        if let Some(provider) = create_github_provider(config).await? {
            providers.insert("github".to_string(), Arc::new(provider));
            count += 1;
        }

        // Generic OIDC
        if let Some(provider) = create_oidc_provider(config).await? {
            let name = provider.name().to_string();
            providers.insert(name.clone(), Arc::new(provider));
            info!("Generic OIDC provider registered as '{}'", name);
            count += 1;
        }

        // Feishu
        if let Some(provider) = create_feishu_provider(config).await? {
            providers.insert("feishu".to_string(), Arc::new(provider));
            count += 1;
        }

        if count == 0 {
            warn!("No OAuth providers configured");
        } else {
            info!("Initialized {} OAuth provider(s)", count);
        }

        Ok(())
    }

    /// Get list of available provider names
    pub async fn get_available_providers(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }

    /// Get a specific provider
    pub async fn get_provider(&self, name: &str) -> AppResult<Arc<dyn OAuthProvider>> {
        let providers = self.providers.read().await;
        providers
            .get(name)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("OAuth provider '{}' not found", name)))
    }

    /// Check if a provider is configured
    pub async fn is_provider_configured(&self, name: &str) -> bool {
        let providers = self.providers.read().await;
        providers.contains_key(name)
    }

    /// Generate state parameter
    fn generate_state() -> String {
        use base64::Engine;
        use rand::Rng;
        let mut rng = rand::rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes)
    }

    /// Store OAuth state
    async fn store_state(&self, state_id: String, state_data: OAuthState) {
        let mut states = self.states.write().await;
        states.insert(state_id, state_data);

        // Clean up old states (older than 10 minutes)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        states.retain(|_, state| current_time - state.created_at < 600);
    }

    /// Retrieve and remove OAuth state
    async fn retrieve_state(&self, state_id: &str) -> Option<OAuthState> {
        let mut states = self.states.write().await;
        states.remove(state_id)
    }

    /// Initiate OAuth login flow
    pub async fn initiate_login(
        &self,
        provider_name: &str,
        redirect_path: Option<String>,
    ) -> AppResult<String> {
        let provider = self.get_provider(provider_name).await?;

        // Generate state
        let state_id = Self::generate_state();

        // Generate PKCE if configured
        let pkce = if self.config.oauth_code_challenge_method.is_some() {
            Some(PKCEData::generate())
        } else {
            None
        };

        // Store state
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.store_state(
            state_id.clone(),
            OAuthState {
                provider: provider_name.to_string(),
                pkce: pkce.clone(),
                redirect_path,
                created_at: current_time,
            },
        )
        .await;

        // Generate authorization URL
        let auth_url = provider
            .get_authorization_url(&state_id, pkce.as_ref())
            .await?;

        debug!("Generated auth URL for {}: {}", provider_name, auth_url);
        Ok(auth_url)
    }

    /// Handle OAuth callback
    pub async fn handle_callback(
        &self,
        code: &str,
        state_id: &str,
    ) -> AppResult<(String, OAuthTokenResponse, OAuthUserInfo)> {
        // Retrieve state
        let state = self
            .retrieve_state(state_id)
            .await
            .ok_or_else(|| AppError::Auth("Invalid or expired OAuth state".to_string()))?;

        debug!("Handling callback for provider: {}", state.provider);

        // Get provider
        let provider = self.get_provider(&state.provider).await?;

        // Exchange code for token
        let pkce_verifier = state.pkce.as_ref().map(|p| p.code_verifier.as_str());
        let token_response = provider.exchange_code(code, pkce_verifier).await?;

        // Get user info
        let user_info = provider.get_user_info(&token_response.access_token).await?;

        debug!(
            "OAuth callback successful for provider {} (user sub: {})",
            state.provider, user_info.sub
        );

        Ok((state.provider, token_response, user_info))
    }

    /// Create OAuth session from token response
    pub async fn create_session(
        &self,
        user_id: &str,
        provider: &str,
        token_response: OAuthTokenResponse,
    ) -> AppResult<()> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Calculate expires_at
        let expires_at = if let Some(expires_in) = token_response.expires_in {
            current_time + expires_in
        } else {
            current_time + 3600 // Default 1 hour
        };

        let token_data = OAuthTokenData {
            access_token: token_response.access_token,
            token_type: token_response.token_type,
            refresh_token: token_response.refresh_token,
            id_token: token_response.id_token,
            expires_in: token_response.expires_in,
            expires_at,
            issued_at: current_time,
            scope: token_response.scope,
        };

        // Delete existing session for this provider/user
        self.session_service
            .delete_session_by_provider_and_user_id(provider, user_id)
            .await?;

        // Create new session
        self.session_service
            .create_session(user_id, provider, token_data)
            .await?;

        info!(
            "Created OAuth session for user {} with provider {}",
            user_id, provider
        );

        Ok(())
    }

    /// Refresh OAuth token if needed
    pub async fn refresh_if_needed(
        &self,
        user_id: &str,
        provider_name: &str,
    ) -> AppResult<Option<OAuthTokenData>> {
        // Get session
        let session = self
            .session_service
            .get_session_by_provider_and_user_id(provider_name, user_id)
            .await?;

        let session = match session {
            Some(s) => s,
            None => return Ok(None),
        };

        // Check if refresh is needed
        if !self.session_service.needs_refresh(&session) {
            return Ok(Some(session.token));
        }

        debug!(
            "Token refresh needed for user {} provider {}",
            user_id, provider_name
        );

        // Check if we have a refresh token
        let refresh_token = match &session.token.refresh_token {
            Some(rt) => rt.clone(),
            None => {
                warn!(
                    "No refresh token available for user {} provider {}",
                    user_id, provider_name
                );
                return Ok(Some(session.token));
            }
        };

        // Get provider
        let provider = self.get_provider(provider_name).await?;

        // Refresh token
        match provider.refresh_token(&refresh_token).await {
            Ok(new_token) => {
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                let expires_at = if let Some(expires_in) = new_token.expires_in {
                    current_time + expires_in
                } else {
                    current_time + 3600
                };

                let mut token_data = OAuthTokenData {
                    access_token: new_token.access_token,
                    token_type: new_token.token_type,
                    refresh_token: new_token.refresh_token,
                    id_token: new_token.id_token,
                    expires_in: new_token.expires_in,
                    expires_at,
                    issued_at: current_time,
                    scope: new_token.scope,
                };

                // Preserve old refresh token if not provided
                if token_data.refresh_token.is_none() {
                    token_data.refresh_token = Some(refresh_token);
                }

                // Update session
                self.session_service
                    .update_session_by_id(&session.id, token_data.clone())
                    .await?;

                info!(
                    "Refreshed token for user {} provider {}",
                    user_id, provider_name
                );

                Ok(Some(token_data))
            }
            Err(e) => {
                error!(
                    "Failed to refresh token for user {} provider {}: {}",
                    user_id, provider_name, e
                );
                // Delete invalid session
                self.session_service
                    .delete_session_by_id(&session.id)
                    .await?;
                Ok(None)
            }
        }
    }

    /// Get valid access token (with automatic refresh)
    pub async fn get_access_token(
        &self,
        user_id: &str,
        provider_name: &str,
    ) -> AppResult<Option<String>> {
        match self.refresh_if_needed(user_id, provider_name).await? {
            Some(token_data) => Ok(Some(token_data.access_token)),
            None => Ok(None),
        }
    }

    /// Extract custom claim from user info
    pub fn extract_claim(
        &self,
        user_info: &OAuthUserInfo,
        claim_path: &str,
    ) -> Option<serde_json::Value> {
        // Support nested claims with dot notation (e.g., "realm_access.roles")
        let parts: Vec<&str> = claim_path.split('.').collect();

        let mut current = serde_json::to_value(user_info).ok()?;

        for part in parts {
            current = current.get(part)?.clone();
        }

        Some(current)
    }

    /// Extract roles from user info
    pub fn extract_roles(&self, user_info: &OAuthUserInfo) -> Vec<String> {
        let claim_path = &self.config.oauth_roles_claim;

        match self.extract_claim(user_info, claim_path) {
            Some(serde_json::Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            Some(serde_json::Value::String(s)) => vec![s],
            _ => vec![],
        }
    }

    /// Extract groups from user info
    pub fn extract_groups(&self, user_info: &OAuthUserInfo) -> Vec<String> {
        let claim_path = &self.config.oauth_groups_claim;

        match self.extract_claim(user_info, claim_path) {
            Some(serde_json::Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            Some(serde_json::Value::String(s)) => vec![s],
            _ => vec![],
        }
    }

    /// Determine user role from OAuth claims
    pub fn determine_user_role(&self, user_info: &OAuthUserInfo, is_first_user: bool) -> String {
        // First user is always admin
        if is_first_user {
            return "admin".to_string();
        }

        // If role management is disabled, use default role
        if !self.config.enable_oauth_role_management {
            return self.config.default_user_role.clone();
        }

        let roles = self.extract_roles(user_info);

        // Check for admin roles
        for admin_role in &self.config.oauth_admin_roles {
            if roles.contains(admin_role) {
                return "admin".to_string();
            }
        }

        // Check for allowed roles
        for allowed_role in &self.config.oauth_allowed_roles {
            if roles.contains(allowed_role) {
                return "user".to_string();
            }
        }

        // Default role
        self.config.default_user_role.clone()
    }

    /// Validate email domain
    pub fn is_email_domain_allowed(&self, email: &str) -> bool {
        // If "*" is in allowed domains, allow all
        if self.config.oauth_allowed_domains.contains(&"*".to_string()) {
            return true;
        }

        // Extract domain from email
        let domain = email.split('@').nth(1).unwrap_or("");

        self.config
            .oauth_allowed_domains
            .contains(&domain.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_nested_claim() {
        let user_info = OAuthUserInfo {
            sub: "123".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: Some(true),
            name: Some("Test User".to_string()),
            given_name: None,
            family_name: None,
            picture: None,
            locale: None,
            extra: {
                let mut map = HashMap::new();
                map.insert(
                    "realm_access".to_string(),
                    serde_json::json!({"roles": ["user", "admin"]}),
                );
                map
            },
        };

        let config = Config::from_env().unwrap();
        let manager = OAuthManager {
            providers: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            session_service: Arc::new(todo!()),
            config,
        };

        let claim = manager.extract_claim(&user_info, "realm_access.roles");
        assert!(claim.is_some());
        assert_eq!(claim.unwrap(), serde_json::json!(["user", "admin"]));
    }
}
