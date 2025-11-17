use libsql::{Builder, Connection, Database as LibsqlDatabase};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Database {
    db: Arc<LibsqlDatabase>,
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        // Parse the database URL to determine if it's local or remote
        let db = if database_url.starts_with("libsql://") || database_url.starts_with("https://") {
            // Remote Turso database
            let auth_token = std::env::var("TURSO_AUTH_TOKEN")
                .unwrap_or_else(|_| "".to_string());
            
            Builder::new_remote(database_url.to_string(), auth_token)
                .build()
                .await?
        } else {
            // Local SQLite database
            Builder::new_local(database_url)
                .build()
                .await?
        };

        let conn = db.connect()?;
        
        Ok(Database { 
            db: Arc::new(db),
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        // Run SQLite migrations in order
        let migrations = vec![
            include_str!("../migrations/sqlite/001_initial.sql"),
            include_str!("../migrations/sqlite/002_add_missing_columns.sql"),
            include_str!("../migrations/sqlite/003_add_config_table.sql"),
            include_str!("../migrations/sqlite/004_add_channel_messages.sql"),
            include_str!("../migrations/sqlite/005_add_note_feedback_tables.sql"),
            include_str!("../migrations/sqlite/006_add_folder_data_column.sql"),
            include_str!("../migrations/sqlite/007_add_file_columns.sql"),
            include_str!("../migrations/sqlite/008_add_group_data_column.sql"),
            include_str!("../migrations/sqlite/009_make_message_chat_id_nullable.sql"),
            include_str!("../migrations/sqlite/010_fix_chat_timestamps.sql"),
        ];

        for (idx, migration_sql) in migrations.iter().enumerate() {
            tracing::info!("Running migration {}", idx + 1);

            // Parse and execute SQL statements
            let statements = Self::parse_sql_statements(migration_sql);
            for statement in statements {
                let trimmed = statement.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("--") {
                    let conn = self.conn.lock().await;
                    match conn.execute(trimmed, ()).await {
                        Ok(_) => {}
                        Err(e) => {
                            // Log error but continue if it's a "already exists" error
                            let error_msg = e.to_string();
                            if error_msg.contains("already exists") {
                                tracing::debug!(
                                    "Skipping non-fatal migration error in migration {}: {}",
                                    idx + 1,
                                    e
                                );
                            } else {
                                tracing::warn!(
                                    "Error in migration {} statement: {} - Error: {}",
                                    idx + 1,
                                    trimmed.chars().take(100).collect::<String>(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("All migrations completed");
        Ok(())
    }

    /// Parse SQL statements, handling SQLite-specific syntax
    fn parse_sql_statements(sql: &str) -> Vec<String> {
        let mut statements = Vec::new();
        let mut current_statement = String::new();

        for line in sql.lines() {
            let trimmed_line = line.trim();

            // Skip empty lines and comments at the start
            if current_statement.is_empty()
                && (trimmed_line.is_empty() || trimmed_line.starts_with("--"))
            {
                continue;
            }

            current_statement.push_str(line);
            current_statement.push('\n');

            if trimmed_line.ends_with(';') {
                // Regular statement ended
                statements.push(current_statement.clone());
                current_statement.clear();
            }
        }

        // Add any remaining statement
        if !current_statement.trim().is_empty() {
            statements.push(current_statement);
        }

        statements
    }

    pub fn pool(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    // User methods
    pub async fn get_user_by_id(
        &self,
        user_id: &str,
    ) -> Result<crate::models::user::User, anyhow::Error> {
        let conn = self.conn.lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, name, email, username, role, profile_image_url, bio, gender, 
                   date_of_birth, info, settings,
                   api_key, oauth_sub, last_active_at, updated_at, created_at
            FROM "user" 
            WHERE id = ?
            "#,
            [user_id],
        ).await?;

        if let Some(row) = rows.next().await? {
            Ok(crate::models::user::User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                username: row.get(3)?,
                role: row.get(4)?,
                profile_image_url: row.get(5)?,
                bio: row.get(6)?,
                gender: row.get(7)?,
                date_of_birth: row.get(8)?,
                info: row.get::<Option<String>>(9)?.and_then(|s| serde_json::from_str(&s).ok()),
                settings: row.get::<Option<String>>(10)?.and_then(|s| serde_json::from_str(&s).ok()),
                api_key: row.get(11)?,
                oauth_sub: row.get(12)?,
                last_active_at: row.get(13)?,
                updated_at: row.get(14)?,
                created_at: row.get(15)?,
            })
        } else {
            Err(anyhow::anyhow!("User not found"))
        }
    }

    pub async fn get_all_users(&self) -> Result<Vec<crate::models::user::User>, anyhow::Error> {
        let conn = self.conn.lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, name, email, username, role, profile_image_url, bio, gender, 
                   date_of_birth, info, settings,
                   api_key, oauth_sub, last_active_at, updated_at, created_at
            FROM "user"
            "#,
            (),
        ).await?;

        let mut users = Vec::new();
        while let Some(row) = rows.next().await? {
            users.push(crate::models::user::User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                username: row.get(3)?,
                role: row.get(4)?,
                profile_image_url: row.get(5)?,
                bio: row.get(6)?,
                gender: row.get(7)?,
                date_of_birth: row.get(8)?,
                info: row.get::<Option<String>>(9)?.and_then(|s| serde_json::from_str(&s).ok()),
                settings: row.get::<Option<String>>(10)?.and_then(|s| serde_json::from_str(&s).ok()),
                api_key: row.get(11)?,
                oauth_sub: row.get(12)?,
                last_active_at: row.get(13)?,
                updated_at: row.get(14)?,
                created_at: row.get(15)?,
            });
        }

        Ok(users)
    }

    // Group methods
    pub async fn get_group_by_id(
        &self,
        group_id: &str,
    ) -> Result<crate::models::group::Group, anyhow::Error> {
        let conn = self.conn.lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, user_id, name, description, 
                   permissions, user_ids, meta, created_at, updated_at
            FROM "group" 
            WHERE id = ?
            "#,
            [group_id],
        ).await?;

        if let Some(row) = rows.next().await? {
            Ok(crate::models::group::Group {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                permissions: row.get::<Option<String>>(4)?.and_then(|s| serde_json::from_str(&s).ok()),
                user_ids: row.get::<Option<String>>(5)?.and_then(|s| serde_json::from_str(&s).ok()),
                meta: row.get::<Option<String>>(6)?.and_then(|s| serde_json::from_str(&s).ok()),
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        } else {
            Err(anyhow::anyhow!("Group not found"))
        }
    }

    pub async fn get_all_groups(&self) -> Result<Vec<crate::models::group::Group>, anyhow::Error> {
        let conn = self.conn.lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, user_id, name, description, 
                   permissions, user_ids, meta, created_at, updated_at
            FROM "group"
            "#,
            (),
        ).await?;

        let mut groups = Vec::new();
        while let Some(row) = rows.next().await? {
            groups.push(crate::models::group::Group {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                permissions: row.get::<Option<String>>(4)?.and_then(|s| serde_json::from_str(&s).ok()),
                user_ids: row.get::<Option<String>>(5)?.and_then(|s| serde_json::from_str(&s).ok()),
                meta: row.get::<Option<String>>(6)?.and_then(|s| serde_json::from_str(&s).ok()),
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            });
        }

        Ok(groups)
    }

    // Model methods
    pub async fn get_model_by_id(
        &self,
        model_id: &str,
    ) -> Result<crate::models::model::Model, anyhow::Error> {
        let conn = self.conn.lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, user_id, base_model_id, name, 
                   params, meta, access_control,
                   is_active, created_at, updated_at
            FROM model 
            WHERE id = ?
            "#,
            [model_id],
        ).await?;

        if let Some(row) = rows.next().await? {
            Ok(crate::models::model::Model {
                id: row.get(0)?,
                user_id: row.get(1)?,
                base_model_id: row.get(2)?,
                name: row.get(3)?,
                params: row.get::<Option<String>>(4)?.and_then(|s| serde_json::from_str(&s).ok()),
                meta: row.get::<Option<String>>(5)?.and_then(|s| serde_json::from_str(&s).ok()),
                access_control: row.get::<Option<String>>(6)?.and_then(|s| serde_json::from_str(&s).ok()),
                is_active: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        } else {
            Err(anyhow::anyhow!("Model not found"))
        }
    }

    pub async fn get_all_models(&self) -> Result<Vec<crate::models::model::Model>, anyhow::Error> {
        let conn = self.conn.lock().await;
        let mut rows = conn.query(
            r#"
            SELECT id, user_id, base_model_id, name, 
                   params, meta, access_control,
                   is_active, created_at, updated_at
            FROM model
            "#,
            (),
        ).await?;

        let mut models = Vec::new();
        while let Some(row) = rows.next().await? {
            models.push(crate::models::model::Model {
                id: row.get(0)?,
                user_id: row.get(1)?,
                base_model_id: row.get(2)?,
                name: row.get(3)?,
                params: row.get::<Option<String>>(4)?.and_then(|s| serde_json::from_str(&s).ok()),
                meta: row.get::<Option<String>>(5)?.and_then(|s| serde_json::from_str(&s).ok()),
                access_control: row.get::<Option<String>>(6)?.and_then(|s| serde_json::from_str(&s).ok()),
                is_active: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            });
        }

        Ok(models)
    }
}
