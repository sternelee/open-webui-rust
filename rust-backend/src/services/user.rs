use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::User;
use crate::utils::time::current_timestamp_seconds;
use chrono::NaiveDate;

pub struct UserService<'a> {
    db: &'a Database,
}

impl<'a> UserService<'a> {
    pub fn new(db: &'a Database) -> Self {
        UserService { db }
    }

    pub async fn get_user_by_id(&self, id: &str) -> AppResult<Option<User>> {
        let conn = self.db.pool().lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, name, email, username, role, profile_image_url, bio, gender, 
                   date_of_birth, 
                   COALESCE(info, '{}') as info, 
                   COALESCE(settings, '{}') as settings, 
                   api_key, oauth_sub, 
                   last_active_at, updated_at, created_at
            FROM "user"
            WHERE id = ?
            "#,
            [id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().await.map_err(|e| AppError::Database(e.to_string()))? {
            Ok(Some(User::from_row(&row).map_err(|e| AppError::Database(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_user_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let conn = self.db.pool().lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, name, email, username, role, profile_image_url, bio, gender, 
                   date_of_birth, 
                   COALESCE(info, '{}') as info, 
                   COALESCE(settings, '{}') as settings, 
                   api_key, oauth_sub, 
                   last_active_at, updated_at, created_at
            FROM "user"
            WHERE email = ?
            "#,
            [email],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().await.map_err(|e| AppError::Database(e.to_string()))? {
            Ok(Some(User::from_row(&row).map_err(|e| AppError::Database(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub async fn get_user_by_api_key(&self, api_key: &str) -> AppResult<Option<User>> {
        let conn = self.db.pool().lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, name, email, username, role, profile_image_url, bio, gender, 
                   date_of_birth, 
                   COALESCE(info, '{}') as info, 
                   COALESCE(settings, '{}') as settings, 
                   api_key, oauth_sub, 
                   last_active_at, updated_at, created_at
            FROM "user"
            WHERE api_key = ?
            "#,
            [api_key],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().await.map_err(|e| AppError::Database(e.to_string()))? {
            Ok(Some(User::from_row(&row).map_err(|e| AppError::Database(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_first_user(&self) -> AppResult<Option<User>> {
        let conn = self.db.pool().lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, name, email, username, role, profile_image_url, bio, gender, 
                   date_of_birth, 
                   COALESCE(info, '{}') as info, 
                   COALESCE(settings, '{}') as settings, 
                   api_key, oauth_sub, 
                   last_active_at, updated_at, created_at
            FROM "user"
            ORDER BY created_at ASC
            LIMIT 1
            "#,
            (),
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().await.map_err(|e| AppError::Database(e.to_string()))? {
            Ok(Some(User::from_row(&row).map_err(|e| AppError::Database(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub async fn create_user(
        &self,
        id: &str,
        name: &str,
        email: &str,
        role: &str,
        profile_image_url: &str,
    ) -> AppResult<User> {
        let now = current_timestamp_seconds();

        let conn = self.db.pool().lock().await;
        conn.execute(
            r#"
            INSERT INTO "user" (id, name, email, role, profile_image_url, last_active_at, updated_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            [id, name, email, role, profile_image_url, &now.to_string(), &now.to_string(), &now.to_string()],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        drop(conn); // Release lock before calling get_user_by_id

        self.get_user_by_id(id)
            .await?
            .ok_or_else(|| AppError::InternalServerError("Failed to create user".to_string()))
    }

    #[allow(dead_code)]
    pub async fn update_user_last_active(&self, id: &str) -> AppResult<()> {
        let now = current_timestamp_seconds();

        let conn = self.db.pool().lock().await;
        conn.execute(
            r#"
            UPDATE "user"
            SET last_active_at = ?
            WHERE id = ?
            "#,
            [&now.to_string(), id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn list_users(&self, skip: i64, limit: i64) -> AppResult<Vec<User>> {
        let conn = self.db.pool().lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, name, email, username, role, profile_image_url, bio, gender, 
                   date_of_birth, 
                   COALESCE(info, '{}') as info, 
                   COALESCE(settings, '{}') as settings, 
                   api_key, oauth_sub, 
                   last_active_at, updated_at, created_at
            FROM "user"
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
            [&limit.to_string(), &skip.to_string()],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        let mut users = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| AppError::Database(e.to_string()))? {
            users.push(User::from_row(&row).map_err(|e| AppError::Database(e.to_string()))?);
        }

        Ok(users)
    }

    pub async fn count_users(&self) -> AppResult<i64> {
        let conn = self.db.pool().lock().await;
        let mut rows = conn.query("SELECT COUNT(*) as count FROM \"user\"", ())
            .await.map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().await.map_err(|e| AppError::Database(e.to_string()))? {
            let count: i64 = row.get(0).map_err(|e| AppError::Database(e.to_string()))?;
            Ok(count)
        } else {
            Ok(0)
        }
    }

    pub async fn get_user_count(&self) -> AppResult<i64> {
        self.count_users().await
    }

    pub async fn update_user_role(&self, id: &str, role: &str) -> AppResult<()> {
        let now = current_timestamp_seconds();
        let conn = self.db.pool().lock().await;
        conn.execute(
            r#"
            UPDATE "user"
            SET role = ?, updated_at = ?
            WHERE id = ?
            "#,
            [role, &now.to_string(), id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn delete_user(&self, id: &str) -> AppResult<()> {
        let conn = self.db.pool().lock().await;
        conn.execute(r#"DELETE FROM "user" WHERE id = ?"#, [id])
            .await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn update_user_settings(
        &self,
        id: &str,
        settings: &serde_json::Value,
    ) -> AppResult<()> {
        let now = current_timestamp_seconds();
        let settings_str = serde_json::to_string(settings)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;

        let conn = self.db.pool().lock().await;
        conn.execute(
            r#"
            UPDATE "user"
            SET settings = ?, updated_at = ?
            WHERE id = ?
            "#,
            [&settings_str, &now.to_string(), id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Update user profile information (name, profile_image_url, bio, gender, date_of_birth)
    pub async fn update_user_profile(
        &self,
        id: &str,
        name: Option<&str>,
        profile_image_url: Option<&str>,
        bio: Option<&str>,
        gender: Option<&str>,
        date_of_birth: Option<NaiveDate>,
    ) -> AppResult<()> {
        let now = current_timestamp_seconds();

        // Build dynamic SQL query based on which fields are provided
        let mut query_parts = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(n) = name {
            query_parts.push("name = ?");
            params.push(n.to_string());
        }
        if let Some(p) = profile_image_url {
            query_parts.push("profile_image_url = ?");
            params.push(p.to_string());
        }
        if let Some(b) = bio {
            query_parts.push("bio = ?");
            params.push(b.to_string());
        }
        if let Some(g) = gender {
            query_parts.push("gender = ?");
            params.push(g.to_string());
        }
        if let Some(d) = date_of_birth {
            query_parts.push("date_of_birth = ?");
            params.push(d.format("%Y-%m-%d").to_string());
        }

        // Always update updated_at
        query_parts.push("updated_at = ?");
        params.push(now.to_string());

        if query_parts.len() == 1 {
            // Only updated_at would be updated, nothing to do
            return Ok(());
        }

        let query_str = format!(
            r#"UPDATE "user" SET {} WHERE id = ?"#,
            query_parts.join(", ")
        );
        params.push(id.to_string());

        let conn = self.db.pool().lock().await;
        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
        conn.execute(&query_str, params_refs.as_slice())
            .await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn get_valid_user_ids(&self, user_ids: &[String]) -> AppResult<Vec<String>> {
        if user_ids.is_empty() {
            return Ok(vec![]);
        }

        // For SQLite, we need to build a query with multiple OR conditions
        // or use IN clause with dynamic placeholders
        let placeholders: Vec<String> = (0..user_ids.len()).map(|_| "?".to_string()).collect();
        let query_str = format!(
            r#"SELECT id FROM "user" WHERE id IN ({})"#,
            placeholders.join(", ")
        );

        let conn = self.db.pool().lock().await;
        let params_refs: Vec<&str> = user_ids.iter().map(|s| s.as_str()).collect();
        let mut rows = conn.query(&query_str, params_refs.as_slice())
            .await.map_err(|e| AppError::Database(e.to_string()))?;

        let mut result = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| AppError::Database(e.to_string()))? {
            let id: String = row.get(0).map_err(|e| AppError::Database(e.to_string()))?;
            result.push(id);
        }

        Ok(result)
    }
}
