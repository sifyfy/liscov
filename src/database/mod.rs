pub mod crud;
pub mod models;
pub mod schema;

pub use models::*;
pub use schema::*;

use anyhow::Result;
use std::path::Path;

/// liscov用データベース接続管理
pub struct LiscovDatabase {
    pub connection: rusqlite::Connection,
    pub schema_version: u32,
}

impl LiscovDatabase {
    /// 新しいデータベース接続を作成
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let connection = rusqlite::Connection::open(db_path)?;
        let mut db = Self {
            connection,
            schema_version: 1,
        };

        db.initialize_schema()?;
        Ok(db)
    }

    /// インメモリデータベースを作成（テスト用）
    pub fn new_in_memory() -> Result<Self> {
        let connection = rusqlite::Connection::open_in_memory()?;
        let mut db = Self {
            connection,
            schema_version: 1,
        };

        db.initialize_schema()?;
        Ok(db)
    }

    /// データベーススキーマを初期化
    fn initialize_schema(&mut self) -> Result<()> {
        self.connection.execute_batch(include_str!("schema.sql"))?;
        tracing::info!("Database schema initialized successfully");
        Ok(())
    }
}
