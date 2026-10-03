//! Database module for Liscov

mod crud;
mod migrations;
pub mod models;
mod reactions;

pub use crud::*;
pub use models::*;
pub use reactions::*;

use anyhow::Result;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Database wrapper for thread-safe access
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// アプリのデータディレクトリの DB を開く
    pub fn new() -> Result<Self> {
        Self::open(&get_database_path()?)
    }

    /// 指定したファイルの DB を開く（無ければ作る）。マイグレーションと、閉じられなかったセッションの回収も行う
    ///
    /// 統合テストは一時ディレクトリの DB をこれで開く。
    pub fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = open_file_database(path)?;

        // Run migrations
        migrations::run_migrations(&conn)?;

        // 前回の強制終了などで閉じられなかったセッションを閉じる（起動を止めるほどの失敗ではない）
        match crud::close_unfinished_sessions(&conn) {
            Ok(0) => {}
            Ok(n) => tracing::info!("前回閉じられなかったセッションを {} 件閉じた", n),
            Err(e) => tracing::warn!("閉じられなかったセッションの回収に失敗: {:#}", e),
        }

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

    /// 所有権付きガードとして接続を取得する
    ///
    /// `Database` を保持するロック（例: `AppState::database` の read guard）を
    /// 解放した後も接続を使い続けられるため、コマンド層のヘルパーから返却できる。
    pub async fn connection_owned(&self) -> tokio::sync::OwnedMutexGuard<Connection> {
        Arc::clone(&self.conn).lock_owned().await
    }
}

/// DB ファイルを開いて接続の設定をする（08_database.md「書き込み」）
///
/// WAL + synchronous = NORMAL にして、コミットのたびに fsync しない。
fn open_file_database(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")?;
    // journal_mode は結果の行を返すので query_row で設定し、WAL になったか確かめる
    let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        tracing::warn!("DB を WAL にできなかった（journal_mode = {}）", mode);
    }
    Ok(conn)
}

/// データベースファイルのパスを返す
fn get_database_path() -> Result<PathBuf> {
    crate::paths::database_path().map_err(|e| anyhow::anyhow!(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 08_database.md「書き込み」: DB を開くと WAL・synchronous = NORMAL になる
    #[test]
    fn file_database_uses_wal_and_normal_sync() {
        let dir = std::env::temp_dir().join("liscov_test_database_open");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let conn = open_file_database(&dir.join("liscov.db")).unwrap();

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode, "wal");
        assert_eq!(synchronous, 1, "NORMAL");
        assert_eq!(foreign_keys, 1);
    }
}
