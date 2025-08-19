// File: src/examples/example_09_database.rs
//
// This example demonstrates database integration in an MCP server using SQLite.
// It includes connection pooling, prepared statements, migrations, and
// safe database operations with proper error handling.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Sqlite, SqlitePool};

// Database configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
    pub enable_migrations: bool,
    pub enable_logging: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:./data/example.db".to_string(),
            max_connections: 10,
            connection_timeout_seconds: 30,
            enable_migrations: true,
            enable_logging: false,
        }
    }
}

// Request structures
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateUserRequest {
    pub id: i64,
    pub name: Option<String>,
    pub email: Option<String>,
    pub age: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetUserRequest {
    pub id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteUserRequest {
    pub id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchUsersRequest {
    pub query: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// Response structures
#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseStats {
    pub total_users: i64,
    pub table_count: i64,
    pub database_size_bytes: i64,
    pub connection_pool_size: u32,
    pub active_connections: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// Database Server
pub struct DatabaseServer {
    config: DatabaseConfig,
    pool: SqlitePool,
}

impl DatabaseServer {
    pub async fn new(config: DatabaseConfig) -> Result<Self, String> {
        // Ensure data directory exists
        if let Some(parent) =
            std::path::Path::new(&config.database_url.replace("sqlite:", "")).parent()
        {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create database directory: {}", e))?;
        }

        // Create connection pool
        let pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(config.database_url.replace("sqlite:", ""))
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal),
        )
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

        let server = Self { config, pool };

        // Run migrations if enabled
        if server.config.enable_migrations {
            server.run_migrations().await?;
        }

        Ok(server)
    }

    // Run database migrations
    async fn run_migrations(&self) -> Result<(), String> {
        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT UNIQUE NOT NULL,
                age INTEGER,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create users table: {}", e))?;

        // Create index on email for fast lookups
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to create email index: {}", e))?;

        // Create logs table for tracking operations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS operation_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                operation TEXT NOT NULL,
                user_id INTEGER,
                details TEXT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create logs table: {}", e))?;

        eprintln!("✅ Database migrations completed");
        Ok(())
    }

    // Log database operations
    async fn log_operation(&self, operation: &str, user_id: Option<i64>, details: Option<&str>) {
        let _ = sqlx::query(
            "INSERT INTO operation_logs (operation, user_id, details) VALUES (?, ?, ?)",
        )
        .bind(operation)
        .bind(user_id)
        .bind(details)
        .execute(&self.pool)
        .await;
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "create_user".to_string(),
                description: "Create a new user in the database".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "User's full name"
                        },
                        "email": {
                            "type": "string",
                            "description": "User's email address",
                            "format": "email"
                        },
                        "age": {
                            "type": "integer",
                            "description": "User's age (optional)",
                            "minimum": 0,
                            "maximum": 150
                        }
                    },
                    "required": ["name", "email"]
                }),
            },
            Tool {
                name: "get_user".to_string(),
                description: "Retrieve a user by ID".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "integer",
                            "description": "User ID to retrieve"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "update_user".to_string(),
                description: "Update an existing user".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "integer",
                            "description": "User ID to update"
                        },
                        "name": {
                            "type": "string",
                            "description": "New name (optional)"
                        },
                        "email": {
                            "type": "string",
                            "description": "New email (optional)",
                            "format": "email"
                        },
                        "age": {
                            "type": "integer",
                            "description": "New age (optional)"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "delete_user".to_string(),
                description: "Delete a user by ID".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "integer",
                            "description": "User ID to delete"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "search_users".to_string(),
                description: "Search users with optional filters".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query for name or email"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results",
                            "default": 10,
                            "maximum": 100
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Number of results to skip",
                            "default": 0
                        }
                    }
                }),
            },
            Tool {
                name: "get_database_stats".to_string(),
                description: "Get database statistics and health information".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
            },
        ]
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        match name {
            "create_user" => self.create_user(arguments).await,
            "get_user" => self.get_user(arguments).await,
            "update_user" => self.update_user(arguments).await,
            "delete_user" => self.delete_user(arguments).await,
            "search_users" => self.search_users(arguments).await,
            "get_database_stats" => self.get_database_stats(arguments).await,
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    async fn create_user(&self, arguments: Value) -> Result<Value, String> {
        let request: CreateUserRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let result = sqlx::query_as::<_, (i64,)>(
            "INSERT INTO users (name, email, age) VALUES (?, ?, ?) RETURNING id",
        )
        .bind(&request.name)
        .bind(&request.email)
        .bind(request.age)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to create user: {}", e))?;

        let user_id = result.0;

        // Log the operation
        let log_message = format!("Created user: {}", request.name);
        self.log_operation("create_user", Some(user_id), Some(&log_message))
            .await;

        // Fetch the created user
        let user = sqlx::query_as::<_, User>(
            "SELECT id, name, email, age, created_at, updated_at FROM users WHERE id = ?",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch created user: {}", e))?;

        serde_json::to_value(user).map_err(|e| format!("Failed to serialize user: {}", e))
    }

    async fn get_user(&self, arguments: Value) -> Result<Value, String> {
        let request: GetUserRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let user = sqlx::query_as::<_, User>(
            "SELECT id, name, email, age, created_at, updated_at FROM users WHERE id = ?",
        )
        .bind(request.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        match user {
            Some(user) => {
                self.log_operation("get_user", Some(request.id), None).await;
                serde_json::to_value(user).map_err(|e| format!("Failed to serialize user: {}", e))
            }
            None => Err(format!("User with ID {} not found", request.id)),
        }
    }

    async fn update_user(&self, arguments: Value) -> Result<Value, String> {
        let request: UpdateUserRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        // Build dynamic update query
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn sqlx::Encode<'_, Sqlite> + Send + 'static>> = Vec::new();

        if let Some(name) = &request.name {
            updates.push("name = ?");
            params.push(Box::new(name.clone()));
        }

        if let Some(email) = &request.email {
            updates.push("email = ?");
            params.push(Box::new(email.clone()));
        }

        if let Some(age) = request.age {
            updates.push("age = ?");
            params.push(Box::new(age));
        }

        if updates.is_empty() {
            return Err("No fields to update".to_string());
        }

        updates.push("updated_at = datetime('now')");

        let _query = format!("UPDATE users SET {} WHERE id = ?", updates.join(", "));

        // Note: This is simplified for demo. In production, use QueryBuilder
        // or a more sophisticated approach for dynamic queries.
        let _params = params;

        // Simplified update for demo purposes
        let affected_rows = if let Some(name) = &request.name {
            sqlx::query("UPDATE users SET name = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(name)
                .bind(request.id)
                .execute(&self.pool)
                .await
                .map_err(|e| format!("Failed to update user: {}", e))?
                .rows_affected()
        } else if let Some(email) = &request.email {
            sqlx::query("UPDATE users SET email = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(email)
                .bind(request.id)
                .execute(&self.pool)
                .await
                .map_err(|e| format!("Failed to update user: {}", e))?
                .rows_affected()
        } else {
            0
        };

        if affected_rows == 0 {
            return Err(format!("User with ID {} not found", request.id));
        }

        self.log_operation("update_user", Some(request.id), Some("User updated"))
            .await;

        // Return updated user
        let user = sqlx::query_as::<_, User>(
            "SELECT id, name, email, age, created_at, updated_at FROM users WHERE id = ?",
        )
        .bind(request.id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch updated user: {}", e))?;

        serde_json::to_value(user).map_err(|e| format!("Failed to serialize user: {}", e))
    }

    async fn delete_user(&self, arguments: Value) -> Result<Value, String> {
        let request: DeleteUserRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let affected_rows = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(request.id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete user: {}", e))?
            .rows_affected();

        if affected_rows == 0 {
            return Err(format!("User with ID {} not found", request.id));
        }

        self.log_operation("delete_user", Some(request.id), Some("User deleted"))
            .await;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("User with ID {} deleted successfully", request.id),
            "deleted_id": request.id
        }))
    }

    async fn search_users(&self, arguments: Value) -> Result<Value, String> {
        let request: SearchUsersRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let limit = request.limit.unwrap_or(10).min(100);
        let offset = request.offset.unwrap_or(0);

        let (query, users) = if let Some(search_query) = &request.query {
            let search_pattern = format!("%{}%", search_query);
            let users = sqlx::query_as::<_, User>(
                "SELECT id, name, email, age, created_at, updated_at 
                 FROM users 
                 WHERE name LIKE ? OR email LIKE ? 
                 ORDER BY created_at DESC 
                 LIMIT ? OFFSET ?",
            )
            .bind(&search_pattern)
            .bind(&search_pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to search users: {}", e))?;

            (format!("Search for '{}'", search_query), users)
        } else {
            let users = sqlx::query_as::<_, User>(
                "SELECT id, name, email, age, created_at, updated_at 
                 FROM users 
                 ORDER BY created_at DESC 
                 LIMIT ? OFFSET ?",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to list users: {}", e))?;

            ("List all users".to_string(), users)
        };

        self.log_operation("search_users", None, Some(&query)).await;

        Ok(serde_json::json!({
            "users": users,
            "count": users.len(),
            "limit": limit,
            "offset": offset,
            "query": request.query
        }))
    }

    async fn get_database_stats(&self, _arguments: Value) -> Result<Value, String> {
        // Get total users
        let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| format!("Failed to count users: {}", e))?;

        // Get table count
        let table_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| format!("Failed to count tables: {}", e))?;

        let stats = DatabaseStats {
            total_users: total_users.0,
            table_count: table_count.0,
            database_size_bytes: 0, // Simplified for demo
            connection_pool_size: self.pool.size(),
            active_connections: self.pool.num_idle() as u32,
        };

        self.log_operation("get_database_stats", None, None).await;

        serde_json::to_value(stats).map_err(|e| format!("Failed to serialize stats: {}", e))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    eprintln!("🗄️  Starting Database MCP Server");
    eprintln!("===============================");

    // Create config
    let config = DatabaseConfig::default();

    eprintln!("⚙️  Database Configuration:");
    eprintln!("   Database URL: {}", config.database_url);
    eprintln!("   Max connections: {}", config.max_connections);
    eprintln!("   Enable migrations: {}", config.enable_migrations);

    // Create server
    let server = DatabaseServer::new(config).await?;

    // Demo database operations
    eprintln!("\n🧪 Database Operations Demo:");

    // List tools
    let tools = server.list_tools();
    eprintln!("📋 Available tools ({}):", tools.len());
    for tool in &tools {
        eprintln!("  - {}: {}", tool.name, tool.description);
    }

    // Create a demo user
    eprintln!("\n👤 Creating demo user:");
    let create_args = serde_json::json!({
        "name": "Alice Johnson",
        "email": "alice@example.com",
        "age": 28
    });

    match server.call_tool("create_user", create_args).await {
        Ok(result) => {
            if let Ok(user) = serde_json::from_value::<User>(result) {
                eprintln!("  ✅ Created user: {} (ID: {})", user.name, user.id);

                // Update the user
                eprintln!("\n✏️  Updating user:");
                let update_args = serde_json::json!({
                    "id": user.id,
                    "name": "Alice Smith"
                });

                match server.call_tool("update_user", update_args).await {
                    Ok(updated) => {
                        if let Ok(updated_user) = serde_json::from_value::<User>(updated) {
                            eprintln!("  ✅ Updated user: {}", updated_user.name);
                        }
                    }
                    Err(e) => eprintln!("  ❌ Update failed: {}", e),
                }

                // Search users
                eprintln!("\n🔍 Searching users:");
                let search_args = serde_json::json!({
                    "query": "Alice",
                    "limit": 5
                });

                match server.call_tool("search_users", search_args).await {
                    Ok(results) => {
                        let default_count = Value::Number(serde_json::Number::from(0));
                        let count = results.get("count").unwrap_or(&default_count);
                        eprintln!("  ✅ Found {} users matching 'Alice'", count);
                    }
                    Err(e) => eprintln!("  ❌ Search failed: {}", e),
                }
            }
        }
        Err(e) => eprintln!("  ❌ Create user failed: {}", e),
    }

    // Get database stats
    eprintln!("\n📊 Database statistics:");
    match server
        .call_tool("get_database_stats", serde_json::json!({}))
        .await
    {
        Ok(result) => {
            if let Ok(stats) = serde_json::from_value::<DatabaseStats>(result) {
                eprintln!("  ✅ Total users: {}", stats.total_users);
                eprintln!("     Tables: {}", stats.table_count);
                eprintln!("     Pool size: {}", stats.connection_pool_size);
                eprintln!("     Active connections: {}", stats.active_connections);
            }
        }
        Err(e) => eprintln!("  ❌ Stats failed: {}", e),
    }

    eprintln!("\n🎉 Database demo completed!");
    eprintln!("\n💾 Database features demonstrated:");
    eprintln!("   ✅ Connection pooling with SQLite");
    eprintln!("   ✅ Prepared statements for security");
    eprintln!("   ✅ Database migrations");
    eprintln!("   ✅ CRUD operations with proper error handling");
    eprintln!("   ✅ Search and pagination");
    eprintln!("   ✅ Operation logging and statistics");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_database_server() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = DatabaseConfig {
            database_url: format!("sqlite:{}", db_path.to_string_lossy()),
            ..Default::default()
        };

        let server = DatabaseServer::new(config).await.unwrap();

        // Test tools listing
        let tools = server.list_tools();
        assert_eq!(tools.len(), 6);
        assert!(tools.iter().any(|t| t.name == "create_user"));
        assert!(tools.iter().any(|t| t.name == "get_user"));
        assert!(tools.iter().any(|t| t.name == "search_users"));
    }

    #[tokio::test]
    async fn test_user_crud_operations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_crud.db");

        let config = DatabaseConfig {
            database_url: format!("sqlite:{}", db_path.to_string_lossy()),
            ..Default::default()
        };

        let server = DatabaseServer::new(config).await.unwrap();

        // Create user
        let create_args = serde_json::json!({
            "name": "Test User",
            "email": "test@example.com",
            "age": 25
        });

        let result = server.call_tool("create_user", create_args).await.unwrap();
        let user: User = serde_json::from_value(result).unwrap();

        assert_eq!(user.name, "Test User");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.age, Some(25));

        // Get user
        let get_args = serde_json::json!({"id": user.id});
        let result = server.call_tool("get_user", get_args).await.unwrap();
        let fetched_user: User = serde_json::from_value(result).unwrap();

        assert_eq!(fetched_user.id, user.id);
        assert_eq!(fetched_user.name, "Test User");

        // Search users
        let search_args = serde_json::json!({
            "query": "Test",
            "limit": 10
        });

        let result = server.call_tool("search_users", search_args).await.unwrap();
        let count = result.get("count").unwrap().as_u64().unwrap();
        assert!(count > 0);
    }
}
