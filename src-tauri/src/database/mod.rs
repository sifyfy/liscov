//! Database module for Liscov

pub mod models;
mod crud;

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

        // Initialize schema
        conn.execute_batch(include_str!("schema.sql"))?;

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
        conn.execute_batch(include_str!("schema.sql"))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Get the connection for operations
    pub async fn connection(&self) -> tokio::sync::MutexGuard<'_, Connection> {
        self.conn.lock().await
    }
}

/// Get the app name for directory paths (can be overridden via LISCOV_APP_NAME env var for testing)
fn get_app_name() -> String {
    std::env::var("LISCOV_APP_NAME").unwrap_or_else(|_| "liscov".to_string())
}

/// Get the database file path
fn get_database_path() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?;

    Ok(data_dir.join(get_app_name()).join("liscov.db"))
}

/// Get the backup directory path
pub fn get_backup_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?;

    Ok(data_dir.join(get_app_name()).join("backups"))
}
