/// OAuth Provider System
/// Handles OAuth 2.0 / OpenID Connect authentication with multiple providers
use crate::config::Config;
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::{debug, error, info};

/// OAuth provider configuration
#[derive(Debug, Clone)]
pub struct OAuthProviderConfig {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorize_url: String,
    pub token_url: String,
    pub userinfo_url: Option<String>,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
    pub discovery_url: Option<String>,
    pub sub_claim: Option<String>,
    pub picture_url: Option<String>,
}

/// OAuth token response from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// User information from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserInfo {
    pub sub: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// OIDC Discovery document
#[derive(Debug, Clone, Deserialize)]
pub struct OIDCDiscovery {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub userinfo_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,
}

/// PKCE (Proof Key for Code Exchange) data
#[derive(Debug, Clone)]
pub struct PKCEData {
    pub code_verifier: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

impl PKCEData {
    /// Generate PKCE challenge data
    pub fn generate() -> Self {
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);

        Self {
            code_verifier,
            code_challenge,
            code_challenge_method: "S256".to_string(),
        }
    }

    fn generate_code_verifier() -> String {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn generate_code_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let result = hasher.finalize();
        URL_SAFE_NO_PAD.encode(result)
    }
}

/// OAuth provider trait
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Get provider configuration
    fn config(&self) -> &OAuthProviderConfig;

    /// Generate authorization URL
    async fn get_authorization_url(
        &self,
        state: &str,
        pkce: Option<&PKCEData>,
    ) -> AppResult<String>;

    /// Exchange authorization code for tokens
    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: Option<&str>,
    ) -> AppResult<OAuthTokenResponse>;

    /// Get user information
    async fn get_user_info(&self, access_token: &str) -> AppResult<OAuthUserInfo>;

    /// Refresh access token
    async fn refresh_token(&self, refresh_token: &str) -> AppResult<OAuthTokenResponse>;
}

/// Base OAuth provider implementation
pub struct BaseOAuthProvider {
    config: OAuthProviderConfig,
    client: Client,
}

impl BaseOAuthProvider {
    pub fn new(config: OAuthProviderConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Discover OIDC endpoints
    pub async fn discover_oidc(&mut self, discovery_url: &str) -> AppResult<()> {
        debug!("Discovering OIDC endpoints from {}", discovery_url);

        let discovery: OIDCDiscovery = self
            .client
            .get(discovery_url)
            .send()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("OIDC discovery failed: {}", e)))?
            .json()
            .await
            .map_err(|e| {
                AppError::ExternalServiceError(format!("Failed to parse OIDC discovery: {}", e))
            })?;

        // Update configuration with discovered endpoints
        self.config.authorize_url = discovery.authorization_endpoint;
        self.config.token_url = discovery.token_endpoint;
        self.config.userinfo_url = discovery.userinfo_endpoint;

        info!(
            "OIDC discovery successful for {} (issuer: {})",
            self.config.name, discovery.issuer
        );

        Ok(())
    }
}

#[async_trait]
impl OAuthProvider for BaseOAuthProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &OAuthProviderConfig {
        &self.config
    }

    async fn get_authorization_url(
        &self,
        state: &str,
        pkce: Option<&PKCEData>,
    ) -> AppResult<String> {
        let mut params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("redirect_uri", self.config.redirect_uri.as_str()),
            ("response_type", "code"),
            ("state", state),
        ];

        // Add scope
        let scope_str = self.config.scopes.join(" ");
        params.push(("scope", &scope_str));

        // Add PKCE if provided
        let code_challenge;
        let code_challenge_method;
        if let Some(pkce_data) = pkce {
            code_challenge = pkce_data.code_challenge.clone();
            code_challenge_method = pkce_data.code_challenge_method.clone();
            params.push(("code_challenge", &code_challenge));
            params.push(("code_challenge_method", &code_challenge_method));
        }

        // Build URL
        let url = reqwest::Url::parse_with_params(&self.config.authorize_url, &params)
            .map_err(|e| AppError::Internal(format!("Failed to build auth URL: {}", e)))?;

        Ok(url.to_string())
    }

    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: Option<&str>,
    ) -> AppResult<OAuthTokenResponse> {
        let mut params = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.config.redirect_uri),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
        ];

        // Add PKCE verifier if provided
        let verifier_str;
        if let Some(verifier) = pkce_verifier {
            verifier_str = verifier.to_string();
            params.push(("code_verifier", &verifier_str));
        }

        debug!("Exchanging code for token with {}", self.config.name);

        let response = self
            .client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                error!("Token exchange failed: {}", e);
                AppError::ExternalServiceError(format!("Token exchange failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Token exchange failed: {} - {}", status, error_text);
            return Err(AppError::ExternalServiceError(format!(
                "Token exchange failed: {} - {}",
                status, error_text
            )));
        }

        let token_response: OAuthTokenResponse = response.json().await.map_err(|e| {
            error!("Failed to parse token response: {}", e);
            AppError::ExternalServiceError(format!("Failed to parse token response: {}", e))
        })?;

        debug!("Token exchange successful for {}", self.config.name);
        Ok(token_response)
    }

    async fn get_user_info(&self, access_token: &str) -> AppResult<OAuthUserInfo> {
        let userinfo_url = self
            .config
            .userinfo_url
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("No userinfo URL configured".to_string()))?;

        debug!("Fetching user info from {}", userinfo_url);

        let response = self
            .client
            .get(userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to fetch user info: {}", e);
                AppError::ExternalServiceError(format!("Failed to fetch user info: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("User info fetch failed: {} - {}", status, error_text);
            return Err(AppError::ExternalServiceError(format!(
                "User info fetch failed: {} - {}",
                status, error_text
            )));
        }

        let user_info: OAuthUserInfo = response.json().await.map_err(|e| {
            error!("Failed to parse user info: {}", e);
            AppError::ExternalServiceError(format!("Failed to parse user info: {}", e))
        })?;

        debug!("User info fetched successfully for sub: {}", user_info.sub);
        Ok(user_info)
    }

    async fn refresh_token(&self, refresh_token: &str) -> AppResult<OAuthTokenResponse> {
        let params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
        ];

        debug!("Refreshing token for {}", self.config.name);

        let response = self
            .client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                error!("Token refresh failed: {}", e);
                AppError::ExternalServiceError(format!("Token refresh failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Token refresh failed: {} - {}", status, error_text);
            return Err(AppError::ExternalServiceError(format!(
                "Token refresh failed: {} - {}",
                status, error_text
            )));
        }

        let token_response: OAuthTokenResponse = response.json().await.map_err(|e| {
            error!("Failed to parse refresh response: {}", e);
            AppError::ExternalServiceError(format!("Failed to parse refresh response: {}", e))
        })?;

        debug!("Token refresh successful for {}", self.config.name);
        Ok(token_response)
    }
}

/// Create Google OAuth provider
pub async fn create_google_provider(config: &Config) -> AppResult<Option<BaseOAuthProvider>> {
    if config.google_client_id.is_empty() || config.google_client_secret.is_empty() {
        return Ok(None);
    }

    let provider_config = OAuthProviderConfig {
        name: "google".to_string(),
        client_id: config.google_client_id.clone(),
        client_secret: config.google_client_secret.clone(),
        authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
        token_url: "https://oauth2.googleapis.com/token".to_string(),
        userinfo_url: Some("https://openidconnect.googleapis.com/v1/userinfo".to_string()),
        scopes: config
            .google_oauth_scope
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
        redirect_uri: config.google_redirect_uri.clone(),
        discovery_url: Some(
            "https://accounts.google.com/.well-known/openid-configuration".to_string(),
        ),
        sub_claim: None,
        picture_url: None,
    };

    let mut provider = BaseOAuthProvider::new(provider_config);

    // Perform OIDC discovery
    if let Some(discovery_url) = provider.config.discovery_url.clone() {
        provider.discover_oidc(&discovery_url).await?;
    }

    info!("Google OAuth provider configured");
    Ok(Some(provider))
}

/// Create Microsoft OAuth provider
pub async fn create_microsoft_provider(config: &Config) -> AppResult<Option<BaseOAuthProvider>> {
    if config.microsoft_client_id.is_empty()
        || config.microsoft_client_secret.is_empty()
        || config.microsoft_client_tenant_id.is_empty()
    {
        return Ok(None);
    }

    let tenant = &config.microsoft_client_tenant_id;
    let login_base = &config.microsoft_client_login_base_url;

    let provider_config = OAuthProviderConfig {
        name: "microsoft".to_string(),
        client_id: config.microsoft_client_id.clone(),
        client_secret: config.microsoft_client_secret.clone(),
        authorize_url: format!("{}/{}/oauth2/v2.0/authorize", login_base, tenant),
        token_url: format!("{}/{}/oauth2/v2.0/token", login_base, tenant),
        userinfo_url: Some("https://graph.microsoft.com/oidc/userinfo".to_string()),
        scopes: config
            .microsoft_oauth_scope
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
        redirect_uri: config.microsoft_redirect_uri.clone(),
        discovery_url: Some(format!(
            "{}/ {}/v2.0/.well-known/openid-configuration",
            login_base, tenant
        )),
        sub_claim: None,
        picture_url: Some(config.microsoft_client_picture_url.clone()),
    };

    let mut provider = BaseOAuthProvider::new(provider_config);

    // Perform OIDC discovery
    if let Some(discovery_url) = provider.config.discovery_url.clone() {
        if let Err(e) = provider.discover_oidc(&discovery_url).await {
            // Microsoft discovery can fail, use hardcoded endpoints as fallback
            debug!(
                "Microsoft OIDC discovery failed, using hardcoded endpoints: {}",
                e
            );
        }
    }

    info!("Microsoft OAuth provider configured");
    Ok(Some(provider))
}

/// Create GitHub OAuth provider
pub async fn create_github_provider(config: &Config) -> AppResult<Option<BaseOAuthProvider>> {
    if config.github_client_id.is_empty() || config.github_client_secret.is_empty() {
        return Ok(None);
    }

    let provider_config = OAuthProviderConfig {
        name: "github".to_string(),
        client_id: config.github_client_id.clone(),
        client_secret: config.github_client_secret.clone(),
        authorize_url: "https://github.com/login/oauth/authorize".to_string(),
        token_url: "https://github.com/login/oauth/access_token".to_string(),
        userinfo_url: Some("https://api.github.com/user".to_string()),
        scopes: config
            .github_client_scope
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
        redirect_uri: config.github_client_redirect_uri.clone(),
        discovery_url: None,
        sub_claim: Some("id".to_string()),
        picture_url: None,
    };

    let provider = BaseOAuthProvider::new(provider_config);

    info!("GitHub OAuth provider configured");
    Ok(Some(provider))
}

/// Create generic OIDC provider
pub async fn create_oidc_provider(config: &Config) -> AppResult<Option<BaseOAuthProvider>> {
    if config.oauth_client_id.is_empty()
        || config.oauth_client_secret.is_empty()
        || config.openid_provider_url.is_empty()
    {
        return Ok(None);
    }

    let provider_config = OAuthProviderConfig {
        name: config.oauth_provider_name.clone(),
        client_id: config.oauth_client_id.clone(),
        client_secret: config.oauth_client_secret.clone(),
        authorize_url: String::new(), // Will be filled by discovery
        token_url: String::new(),     // Will be filled by discovery
        userinfo_url: None,           // Will be filled by discovery
        scopes: config
            .oauth_scopes
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
        redirect_uri: config.openid_redirect_uri.clone(),
        discovery_url: Some(config.openid_provider_url.clone()),
        sub_claim: config.oauth_sub_claim.clone(),
        picture_url: None,
    };

    let mut provider = BaseOAuthProvider::new(provider_config);

    // Perform OIDC discovery (required for generic OIDC)
    if let Some(discovery_url) = provider.config.discovery_url.clone() {
        provider.discover_oidc(&discovery_url).await?;
    }

    info!(
        "Generic OIDC provider configured: {}",
        config.oauth_provider_name
    );
    Ok(Some(provider))
}

/// Create Feishu OAuth provider
pub async fn create_feishu_provider(config: &Config) -> AppResult<Option<BaseOAuthProvider>> {
    if config.feishu_client_id.is_empty() || config.feishu_client_secret.is_empty() {
        return Ok(None);
    }

    let provider_config = OAuthProviderConfig {
        name: "feishu".to_string(),
        client_id: config.feishu_client_id.clone(),
        client_secret: config.feishu_client_secret.clone(),
        authorize_url: "https://open.feishu.cn/open-apis/authen/v1/authorize".to_string(),
        token_url: "https://open.feishu.cn/open-apis/authen/v2/oauth/token".to_string(),
        userinfo_url: Some("https://open.feishu.cn/open-apis/authen/v1/user_info".to_string()),
        scopes: config
            .feishu_oauth_scope
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
        redirect_uri: config.feishu_redirect_uri.clone(),
        discovery_url: None,
        sub_claim: Some("user_id".to_string()),
        picture_url: None,
    };

    let provider = BaseOAuthProvider::new(provider_config);

    info!("Feishu OAuth provider configured");
    Ok(Some(provider))
}
