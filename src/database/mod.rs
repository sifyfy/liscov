pub mod crud;
pub mod models;
pub mod schema;

pub use crud::{
    delete_viewer_custom_info, get_all_viewer_custom_info_for_broadcaster, get_viewer_custom_info,
    upsert_viewer_custom_info,
};
pub use models::*;
pub use schema::*;

use anyhow::Result;
use directories::ProjectDirs;
use std::path::Path;
use std::path::PathBuf;

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

/// XDGデータディレクトリからデータベースパスを取得
pub fn get_database_path() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "sifyfy", "liscov")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

    let data_dir = project_dirs.data_dir();
    std::fs::create_dir_all(data_dir)?;

    Ok(data_dir.join("liscov.db"))
}

/// データベース接続を取得（非同期ラッパー）
///
/// XDGデータディレクトリにあるliscov.dbに接続し、
/// スキーマが存在しなければ初期化する。
pub async fn get_connection() -> Result<rusqlite::Connection> {
    let db_path = get_database_path()?;

    // 接続を開く
    let conn = rusqlite::Connection::open(&db_path)?;

    // スキーマを初期化（既存の場合はスキップされる）
    conn.execute_batch(include_str!("schema.sql"))?;

    tracing::debug!("Database connection opened: {:?}", db_path);
    Ok(conn)
}
