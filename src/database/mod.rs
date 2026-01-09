pub mod crud;
pub mod models;
pub mod schema;

pub use crud::{
    delete_broadcaster_data, delete_viewer_custom_info, delete_viewer_data,
    get_all_viewer_custom_info_for_broadcaster, get_broadcaster_profile,
    get_distinct_broadcaster_channels, get_viewer_count_for_broadcaster, get_viewer_custom_info,
    get_viewers_for_broadcaster, update_viewer_profile_metadata, upsert_broadcaster_profile,
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

/// バックアップディレクトリのパスを取得
pub fn get_backup_dir() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "sifyfy", "liscov")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

    let backup_dir = project_dirs.data_dir().join("backups");
    std::fs::create_dir_all(&backup_dir)?;

    Ok(backup_dir)
}

/// データベースのバックアップを作成
///
/// タイムスタンプ付きのファイル名でバックアップを作成し、
/// バックアップファイルのパスを返す。
pub fn create_backup() -> Result<PathBuf> {
    let db_path = get_database_path()?;
    let backup_dir = get_backup_dir()?;

    // データベースファイルが存在しない場合はエラー
    if !db_path.exists() {
        return Err(anyhow::anyhow!("Database file does not exist"));
    }

    // タイムスタンプ付きのバックアップファイル名を生成
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("liscov_backup_{}.db", timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    // ファイルをコピー
    std::fs::copy(&db_path, &backup_path)?;

    tracing::info!("📦 Database backup created: {:?}", backup_path);

    Ok(backup_path)
}

/// バックアップ情報
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// バックアップファイルのパス
    pub path: PathBuf,
    /// ファイル名
    pub filename: String,
    /// 作成日時
    pub created_at: chrono::DateTime<chrono::Local>,
    /// ファイルサイズ（バイト）
    pub size: u64,
}

/// 既存のバックアップ一覧を取得（新しい順）
pub fn list_backups() -> Result<Vec<BackupInfo>> {
    let backup_dir = get_backup_dir()?;

    let mut backups = Vec::new();

    if backup_dir.exists() {
        for entry in std::fs::read_dir(&backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with("liscov_backup_") && filename.ends_with(".db") {
                        if let Ok(metadata) = entry.metadata() {
                            let created_at = metadata
                                .modified()
                                .map(|t| chrono::DateTime::<chrono::Local>::from(t))
                                .unwrap_or_else(|_| chrono::Local::now());

                            backups.push(BackupInfo {
                                path: path.clone(),
                                filename: filename.to_string(),
                                created_at,
                                size: metadata.len(),
                            });
                        }
                    }
                }
            }
        }
    }

    // 新しい順にソート
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(backups)
}

/// 指定したパスにバックアップを作成（テスト用）
///
/// 本番環境では `create_backup()` を使用してください。
pub fn create_backup_to_path(source_path: &Path, backup_dir: &Path) -> Result<PathBuf> {
    if !source_path.exists() {
        return Err(anyhow::anyhow!("Source database file does not exist"));
    }

    std::fs::create_dir_all(backup_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("liscov_backup_{}.db", timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    std::fs::copy(source_path, &backup_path)?;

    Ok(backup_path)
}

/// 指定したディレクトリからバックアップ一覧を取得（テスト用）
///
/// 本番環境では `list_backups()` を使用してください。
pub fn list_backups_from_dir(backup_dir: &Path) -> Result<Vec<BackupInfo>> {
    let mut backups = Vec::new();

    if backup_dir.exists() {
        for entry in std::fs::read_dir(backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with("liscov_backup_") && filename.ends_with(".db") {
                        if let Ok(metadata) = entry.metadata() {
                            let created_at = metadata
                                .modified()
                                .map(|t| chrono::DateTime::<chrono::Local>::from(t))
                                .unwrap_or_else(|_| chrono::Local::now());

                            backups.push(BackupInfo {
                                path: path.clone(),
                                filename: filename.to_string(),
                                created_at,
                                size: metadata.len(),
                            });
                        }
                    }
                }
            }
        }
    }

    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(backups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_backup_success() {
        // テスト用の一時ディレクトリを作成
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let backup_dir = temp_dir.path().join("backups");
        std::fs::create_dir_all(&source_dir).unwrap();

        // ソースDBファイルを作成
        let source_path = source_dir.join("test.db");
        std::fs::write(&source_path, b"test database content").unwrap();

        // バックアップを作成
        let result = create_backup_to_path(&source_path, &backup_dir);

        assert!(result.is_ok());
        let backup_path = result.unwrap();

        // バックアップファイルが存在することを確認
        assert!(backup_path.exists());

        // ファイル名のフォーマットを確認
        let filename = backup_path.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with("liscov_backup_"));
        assert!(filename.ends_with(".db"));

        // 内容が同じことを確認
        let backup_content = std::fs::read(&backup_path).unwrap();
        assert_eq!(backup_content, b"test database content");
    }

    #[test]
    fn test_create_backup_source_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("nonexistent.db");
        let backup_dir = temp_dir.path().join("backups");

        let result = create_backup_to_path(&source_path, &backup_dir);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_list_backups_empty() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        std::fs::create_dir_all(&backup_dir).unwrap();

        let result = list_backups_from_dir(&backup_dir);

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_list_backups_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        std::fs::create_dir_all(&backup_dir).unwrap();

        // バックアップファイルを作成
        let backup1 = backup_dir.join("liscov_backup_20250101_120000.db");
        let backup2 = backup_dir.join("liscov_backup_20250102_120000.db");
        let not_backup = backup_dir.join("other_file.txt");

        std::fs::write(&backup1, b"backup1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(&backup2, b"backup2").unwrap();
        std::fs::write(&not_backup, b"not a backup").unwrap();

        let result = list_backups_from_dir(&backup_dir);

        assert!(result.is_ok());
        let backups = result.unwrap();

        // バックアップファイルのみがリストされる
        assert_eq!(backups.len(), 2);

        // ファイル名を確認
        let filenames: Vec<&str> = backups.iter().map(|b| b.filename.as_str()).collect();
        assert!(filenames.contains(&"liscov_backup_20250101_120000.db"));
        assert!(filenames.contains(&"liscov_backup_20250102_120000.db"));

        // other_file.txt は含まれない
        assert!(!filenames.contains(&"other_file.txt"));
    }

    #[test]
    fn test_list_backups_sorted_by_date() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        std::fs::create_dir_all(&backup_dir).unwrap();

        // バックアップファイルを作成（古い順）
        let backup1 = backup_dir.join("liscov_backup_20250101_120000.db");
        std::fs::write(&backup1, b"backup1").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        let backup2 = backup_dir.join("liscov_backup_20250102_120000.db");
        std::fs::write(&backup2, b"backup2").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        let backup3 = backup_dir.join("liscov_backup_20250103_120000.db");
        std::fs::write(&backup3, b"backup3").unwrap();

        let result = list_backups_from_dir(&backup_dir);

        assert!(result.is_ok());
        let backups = result.unwrap();

        assert_eq!(backups.len(), 3);

        // 新しい順にソートされていることを確認（ファイルの更新日時でソート）
        assert!(backups[0].created_at >= backups[1].created_at);
        assert!(backups[1].created_at >= backups[2].created_at);
    }

    #[test]
    fn test_list_backups_nonexistent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("nonexistent");

        let result = list_backups_from_dir(&backup_dir);

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_backup_info_size() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        std::fs::create_dir_all(&backup_dir).unwrap();

        // 特定サイズのバックアップファイルを作成
        let content = vec![0u8; 1024]; // 1KB
        let backup_path = backup_dir.join("liscov_backup_20250101_120000.db");
        std::fs::write(&backup_path, &content).unwrap();

        let result = list_backups_from_dir(&backup_dir);

        assert!(result.is_ok());
        let backups = result.unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].size, 1024);
    }
}
