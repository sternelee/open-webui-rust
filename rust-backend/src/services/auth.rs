use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::Auth;
use crate::utils::password::{hash_password, verify_password};
use crate::utils::time::current_timestamp_seconds;

pub struct AuthService<'a> {
    db: &'a Database,
}

impl<'a> AuthService<'a> {
    pub fn new(db: &'a Database) -> Self {
        AuthService { db }
    }

    pub async fn create_auth(&self, id: &str, email: &str, password: &str) -> AppResult<()> {
        let password_hash = hash_password(password)?;
        let now = current_timestamp_seconds();

        let conn = self.db.pool().lock().await;
        conn.execute(
            r#"
            INSERT INTO auth (id, email, password, active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            [id, email, &password_hash, "1", &now.to_string(), &now.to_string()],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn get_auth_by_email(&self, email: &str) -> AppResult<Option<Auth>> {
        let conn = self.db.pool().lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, email, password, active, created_at, updated_at
            FROM auth
            WHERE email = ?
            "#,
            [email],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().await.map_err(|e| AppError::Database(e.to_string()))? {
            Ok(Some(Auth::from_row(&row).map_err(|e| AppError::Database(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub async fn authenticate(&self, email: &str, password: &str) -> AppResult<Option<String>> {
        let auth = self.get_auth_by_email(email).await?;

        if let Some(auth) = auth {
            if !auth.active {
                return Err(AppError::Unauthorized("Account is not active".to_string()));
            }

            if verify_password(password, &auth.password)? {
                Ok(Some(auth.id))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub async fn update_password(&self, id: &str, new_password: &str) -> AppResult<()> {
        let password_hash = hash_password(new_password)?;
        let now = current_timestamp_seconds();

        let conn = self.db.pool().lock().await;
        conn.execute(
            r#"
            UPDATE auth
            SET password = ?, updated_at = ?
            WHERE id = ?
            "#,
            [&password_hash, &now.to_string(), id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn delete_auth(&self, id: &str) -> AppResult<()> {
        let conn = self.db.pool().lock().await;
        conn.execute("DELETE FROM auth WHERE id = ?", [id])
            .await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}
