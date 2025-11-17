/// OAuth Session Management Service
/// Handles CRUD operations for OAuth sessions with token encryption/decryption
/// Compatible with Python backend implementation
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::oauth_session::{
    OAuthSession, OAuthSessionResponse, OAuthSessionWithToken, OAuthTokenData,
};
use crate::utils::fernet::Fernet;
use chrono::Utc;
use sqlx::Row;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct OAuthSessionService {
    db: Database,
    fernet: Fernet,
}

impl OAuthSessionService {
    /// Create a new OAuth session service
    pub fn new(db: Database, encryption_key: &str) -> AppResult<Self> {
        let fernet = Fernet::new(encryption_key)?;
        Ok(Self { db, fernet })
    }

    /// Get current Unix timestamp
    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    /// Create a new OAuth session
    pub async fn create_session(
        &self,
        user_id: &str,
        provider: &str,
        token_data: OAuthTokenData,
    ) -> AppResult<OAuthSessionWithToken> {
        let session_id = Uuid::new_v4().to_string();
        let current_time = Self::current_timestamp();

        // Encrypt token data
        let encrypted_token = self.fernet.encrypt_json(&token_data)?;

        // Insert into database
        let query = r#"
            INSERT INTO oauth_session (id, user_id, provider, token, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, user_id, provider, token, expires_at, created_at, updated_at
        "#;

        let row = sqlx::query(query)
            .bind(&session_id)
            .bind(user_id)
            .bind(provider)
            .bind(&encrypted_token)
            .bind(token_data.expires_at)
            .bind(current_time)
            .bind(current_time)
            .fetch_one(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to create OAuth session: {}", e);
                AppError::InternalServerError(format!("Failed to create OAuth session: {}", e))
            })?;

        info!(
            "Created OAuth session {} for user {} with provider {}",
            session_id, user_id, provider
        );

        Ok(OAuthSessionWithToken {
            id: row.get("id"),
            user_id: row.get("user_id"),
            provider: row.get("provider"),
            token: token_data,
            expires_at: row.get("expires_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Get session by ID
    pub async fn get_session_by_id(
        &self,
        session_id: &str,
    ) -> AppResult<Option<OAuthSessionWithToken>> {
        let query = r#"
            SELECT id, user_id, provider, token, expires_at, created_at, updated_at
            FROM oauth_session
            WHERE id = $1
        "#;

        let row = sqlx::query(query)
            .bind(session_id)
            .fetch_optional(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch OAuth session: {}", e);
                AppError::InternalServerError(format!("Failed to fetch OAuth session: {}", e))
            })?;

        match row {
            Some(row) => {
                let encrypted_token: String = row.get("token");
                let token_data: OAuthTokenData = self.fernet.decrypt_json(&encrypted_token)?;

                Ok(Some(OAuthSessionWithToken {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    provider: row.get("provider"),
                    token: token_data,
                    expires_at: row.get("expires_at"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                }))
            }
            None => Ok(None),
        }
    }

    /// Get session by ID and user ID (for security)
    pub async fn get_session_by_id_and_user_id(
        &self,
        session_id: &str,
        user_id: &str,
    ) -> AppResult<Option<OAuthSessionWithToken>> {
        let query = r#"
            SELECT id, user_id, provider, token, expires_at, created_at, updated_at
            FROM oauth_session
            WHERE id = $1 AND user_id = $2
        "#;

        let row = sqlx::query(query)
            .bind(session_id)
            .bind(user_id)
            .fetch_optional(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch OAuth session: {}", e);
                AppError::InternalServerError(format!("Failed to fetch OAuth session: {}", e))
            })?;

        match row {
            Some(row) => {
                let encrypted_token: String = row.get("token");
                let token_data: OAuthTokenData = self.fernet.decrypt_json(&encrypted_token)?;

                Ok(Some(OAuthSessionWithToken {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    provider: row.get("provider"),
                    token: token_data,
                    expires_at: row.get("expires_at"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                }))
            }
            None => Ok(None),
        }
    }

    /// Get session by provider and user ID
    pub async fn get_session_by_provider_and_user_id(
        &self,
        provider: &str,
        user_id: &str,
    ) -> AppResult<Option<OAuthSessionWithToken>> {
        let query = r#"
            SELECT id, user_id, provider, token, expires_at, created_at, updated_at
            FROM oauth_session
            WHERE provider = $1 AND user_id = $2
        "#;

        let row = sqlx::query(query)
            .bind(provider)
            .bind(user_id)
            .fetch_optional(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch OAuth session: {}", e);
                AppError::InternalServerError(format!("Failed to fetch OAuth session: {}", e))
            })?;

        match row {
            Some(row) => {
                let encrypted_token: String = row.get("token");
                let token_data: OAuthTokenData = self.fernet.decrypt_json(&encrypted_token)?;

                Ok(Some(OAuthSessionWithToken {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    provider: row.get("provider"),
                    token: token_data,
                    expires_at: row.get("expires_at"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                }))
            }
            None => Ok(None),
        }
    }

    /// Get all sessions for a user
    pub async fn get_sessions_by_user_id(
        &self,
        user_id: &str,
    ) -> AppResult<Vec<OAuthSessionWithToken>> {
        let query = r#"
            SELECT id, user_id, provider, token, expires_at, created_at, updated_at
            FROM oauth_session
            WHERE user_id = $1
            ORDER BY created_at DESC
        "#;

        let rows = sqlx::query(query)
            .bind(user_id)
            .fetch_all(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch OAuth sessions: {}", e);
                AppError::InternalServerError(format!("Failed to fetch OAuth sessions: {}", e))
            })?;

        let mut sessions = Vec::new();
        for row in rows {
            let encrypted_token: String = row.get("token");
            match self.fernet.decrypt_json::<OAuthTokenData>(&encrypted_token) {
                Ok(token_data) => {
                    sessions.push(OAuthSessionWithToken {
                        id: row.get("id"),
                        user_id: row.get("user_id"),
                        provider: row.get("provider"),
                        token: token_data,
                        expires_at: row.get("expires_at"),
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                    });
                }
                Err(e) => {
                    warn!("Failed to decrypt session token, skipping: {}", e);
                    continue;
                }
            }
        }

        Ok(sessions)
    }

    /// Get sessions as response (without decrypted tokens)
    pub async fn get_sessions_response_by_user_id(
        &self,
        user_id: &str,
    ) -> AppResult<Vec<OAuthSessionResponse>> {
        let query = r#"
            SELECT id, user_id, provider, expires_at
            FROM oauth_session
            WHERE user_id = $1
            ORDER BY created_at DESC
        "#;

        let rows = sqlx::query(query)
            .bind(user_id)
            .fetch_all(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch OAuth sessions: {}", e);
                AppError::InternalServerError(format!("Failed to fetch OAuth sessions: {}", e))
            })?;

        let sessions = rows
            .iter()
            .map(|row| OAuthSessionResponse {
                id: row.get("id"),
                user_id: row.get("user_id"),
                provider: row.get("provider"),
                expires_at: row.get("expires_at"),
            })
            .collect();

        Ok(sessions)
    }

    /// Update session tokens
    pub async fn update_session_by_id(
        &self,
        session_id: &str,
        token_data: OAuthTokenData,
    ) -> AppResult<OAuthSessionWithToken> {
        let current_time = Self::current_timestamp();
        let encrypted_token = self.fernet.encrypt_json(&token_data)?;

        let query = r#"
            UPDATE oauth_session
            SET token = $1, expires_at = $2, updated_at = $3
            WHERE id = $4
            RETURNING id, user_id, provider, token, expires_at, created_at, updated_at
        "#;

        let row = sqlx::query(query)
            .bind(&encrypted_token)
            .bind(token_data.expires_at)
            .bind(current_time)
            .bind(session_id)
            .fetch_one(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to update OAuth session: {}", e);
                AppError::InternalServerError(format!("Failed to update OAuth session: {}", e))
            })?;

        debug!("Updated OAuth session {}", session_id);

        Ok(OAuthSessionWithToken {
            id: row.get("id"),
            user_id: row.get("user_id"),
            provider: row.get("provider"),
            token: token_data,
            expires_at: row.get("expires_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Delete session by ID
    pub async fn delete_session_by_id(&self, session_id: &str) -> AppResult<bool> {
        let query = "DELETE FROM oauth_session WHERE id = $1";

        let result = sqlx::query(query)
            .bind(session_id)
            .execute(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to delete OAuth session: {}", e);
                AppError::InternalServerError(format!("Failed to delete OAuth session: {}", e))
            })?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            info!("Deleted OAuth session {}", session_id);
        }

        Ok(deleted)
    }

    /// Delete all sessions for a user
    pub async fn delete_sessions_by_user_id(&self, user_id: &str) -> AppResult<u64> {
        let query = "DELETE FROM oauth_session WHERE user_id = $1";

        let result = sqlx::query(query)
            .bind(user_id)
            .execute(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to delete OAuth sessions: {}", e);
                AppError::InternalServerError(format!("Failed to delete OAuth sessions: {}", e))
            })?;

        let count = result.rows_affected();
        if count > 0 {
            info!("Deleted {} OAuth sessions for user {}", count, user_id);
        }

        Ok(count)
    }

    /// Delete specific provider session for a user
    pub async fn delete_session_by_provider_and_user_id(
        &self,
        provider: &str,
        user_id: &str,
    ) -> AppResult<bool> {
        let query = "DELETE FROM oauth_session WHERE provider = $1 AND user_id = $2";

        let result = sqlx::query(query)
            .bind(provider)
            .bind(user_id)
            .execute(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to delete OAuth session: {}", e);
                AppError::InternalServerError(format!("Failed to delete OAuth session: {}", e))
            })?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            info!(
                "Deleted OAuth session for provider {} and user {}",
                provider, user_id
            );
        }

        Ok(deleted)
    }

    /// Check if session is expired
    pub fn is_session_expired(&self, session: &OAuthSessionWithToken) -> bool {
        let current_time = Self::current_timestamp();
        session.expires_at <= current_time
    }

    /// Check if session needs refresh (expires in less than 5 minutes)
    pub fn needs_refresh(&self, session: &OAuthSessionWithToken) -> bool {
        let current_time = Self::current_timestamp();
        let five_minutes = 5 * 60;
        session.expires_at <= (current_time + five_minutes)
    }

    /// Get valid token for user/provider (checks expiry, returns None if expired)
    pub async fn get_valid_token(
        &self,
        user_id: &str,
        provider: &str,
    ) -> AppResult<Option<OAuthTokenData>> {
        match self
            .get_session_by_provider_and_user_id(provider, user_id)
            .await?
        {
            Some(session) => {
                if self.is_session_expired(&session) {
                    debug!("Session expired for user {} provider {}", user_id, provider);
                    // Delete expired session
                    self.delete_session_by_id(&session.id).await?;
                    Ok(None)
                } else {
                    Ok(Some(session.token))
                }
            }
            None => Ok(None),
        }
    }

    /// Clean up expired sessions (should be run periodically)
    pub async fn cleanup_expired_sessions(&self) -> AppResult<u64> {
        let current_time = Self::current_timestamp();
        let query = "DELETE FROM oauth_session WHERE expires_at <= $1";

        let result = sqlx::query(query)
            .bind(current_time)
            .execute(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to cleanup expired sessions: {}", e);
                AppError::InternalServerError(format!("Failed to cleanup expired sessions: {}", e))
            })?;

        let count = result.rows_affected();
        if count > 0 {
            info!("Cleaned up {} expired OAuth sessions", count);
        }

        Ok(count)
    }

    /// Get sessions expiring soon (for proactive refresh)
    pub async fn get_sessions_expiring_soon(
        &self,
        minutes: i64,
    ) -> AppResult<Vec<OAuthSessionWithToken>> {
        let current_time = Self::current_timestamp();
        let threshold = current_time + (minutes * 60);

        let query = r#"
            SELECT id, user_id, provider, token, expires_at, created_at, updated_at
            FROM oauth_session
            WHERE expires_at > $1 AND expires_at <= $2
            ORDER BY expires_at ASC
        "#;

        let rows = sqlx::query(query)
            .bind(current_time)
            .bind(threshold)
            .fetch_all(&self.db.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch expiring sessions: {}", e);
                AppError::InternalServerError(format!("Failed to fetch expiring sessions: {}", e))
            })?;

        let mut sessions = Vec::new();
        for row in rows {
            let encrypted_token: String = row.get("token");
            match self.fernet.decrypt_json::<OAuthTokenData>(&encrypted_token) {
                Ok(token_data) => {
                    sessions.push(OAuthSessionWithToken {
                        id: row.get("id"),
                        user_id: row.get("user_id"),
                        provider: row.get("provider"),
                        token: token_data,
                        expires_at: row.get("expires_at"),
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                    });
                }
                Err(e) => {
                    warn!("Failed to decrypt session token, skipping: {}", e);
                    continue;
                }
            }
        }

        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a database connection
    // Run with: cargo test --features test-db

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_oauth_session_crud() {
        // This would need database setup
        // Left as example for integration tests
    }
}
