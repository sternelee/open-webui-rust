use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// OAuth Session - matches Python backend schema
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthSession {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub token: String,   // Encrypted JSON
    pub expires_at: i64, // Unix timestamp
    pub created_at: i64, // Unix timestamp
    pub updated_at: i64, // Unix timestamp
}

/// OAuth Session Model for API responses (without encrypted token)
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthSessionResponse {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub expires_at: i64,
}

/// Decrypted token data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenData {
    pub access_token: String,
    pub token_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
    pub expires_at: i64,
    pub issued_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// OAuth Session with decrypted token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthSessionWithToken {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub token: OAuthTokenData,
    pub expires_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Create OAuth Session Request
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOAuthSession {
    pub user_id: String,
    pub provider: String,
    pub token: OAuthTokenData,
}
