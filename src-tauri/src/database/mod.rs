//! Database module for Liscov

pub mod models;
mod crud;
mod migrations;

pub use models::*;
pub use crud::*;

use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Database wrapper for thread-safe access
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create a new database connection
    pub fn new() -> Result<Self> {
        let path = get_database_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        // Run migrations
        migrations::run_migrations(&conn)?;

        tracing::info!("Database initialized at {:?}", path);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory database (for testing)
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrations::run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Get the connection for operations
    pub async fn connection(&self) -> tokio::sync::MutexGuard<'_, Connection> {
        self.conn.lock().await
    }
}

/// データベースファイルのパスを返す
fn get_database_path() -> Result<PathBuf> {
    crate::paths::database_path().map_err(|e| anyhow::anyhow!(e))
}

/// バックアップディレクトリのパスを返す
pub fn get_backup_dir() -> Result<PathBuf> {
    crate::paths::backup_dir().map_err(|e| anyhow::anyhow!(e))
}
