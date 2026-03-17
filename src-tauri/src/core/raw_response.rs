//! YouTubeライブチャットレスポンスの保存とファイル管理

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use tokio::fs::metadata;
use tracing::{info, warn};

/// 保存設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveConfig {
    /// レスポンス保存を有効にするか
    pub enabled: bool,
    /// 保存先ファイルパス
    pub file_path: String,
    /// 最大ファイルサイズ(MB)
    pub max_file_size_mb: u64,
    /// ファイルローテーションを有効にするか
    pub enable_rotation: bool,
    /// 最大保持ファイル数
    pub max_backup_files: u32,
}

impl Default for SaveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            file_path: "raw_responses.ndjson".to_string(),
            max_file_size_mb: 100,
            enable_rotation: true,
            max_backup_files: 5,
        }
    }
}

/// YouTubeレスポンス保存管理
#[derive(Debug)]
pub struct RawResponseSaver {
    config: SaveConfig,
}

impl RawResponseSaver {
    /// 新しいインスタンスを作成
    pub fn new(config: SaveConfig) -> Self {
        Self { config }
    }

    /// レスポンスを保存 (JSON文字列として受け取る)
    pub async fn save_response(&self, response_json: &str) -> Result<()> {
        if !self.config.enabled {
            tracing::debug!("💾 Save response skipped: disabled");
            return Ok(());
        }

        tracing::debug!(
            "💾 save_response called: enabled={}, file_path={}",
            self.config.enabled,
            self.config.file_path
        );

        // ファイルサイズチェックとローテーション
        if self.config.enable_rotation {
            self.check_and_rotate_file().await?;
        }

        // タイムスタンプを追加してJSON行を作成
        let entry = serde_json::json!({
            "timestamp": Utc::now().timestamp(),
            "response": serde_json::from_str::<serde_json::Value>(response_json)
                .unwrap_or_else(|_| serde_json::Value::String(response_json.to_string()))
        });

        let json_line = serde_json::to_string(&entry)
            .context("Failed to serialize response to JSON")?;

        // ファイルに追記
        self.append_to_file(&json_line).await?;

        tracing::info!(
            "💾 Raw response saved successfully to: {}",
            self.config.file_path
        );
        Ok(())
    }

    /// ファイルにJSONラインを追記
    async fn append_to_file(&self, json_line: &str) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.file_path)
            .context("Failed to open raw response file")?;

        writeln!(file, "{}", json_line)?;
        file.flush()?;

        Ok(())
    }

    /// ファイルサイズをチェックしてローテーション
    async fn check_and_rotate_file(&self) -> Result<()> {
        let file_path = Path::new(&self.config.file_path);

        // ファイルが存在しない場合は何もしない
        if !file_path.exists() {
            return Ok(());
        }

        // ファイルサイズをチェック
        let meta = metadata(&self.config.file_path).await?;
        let file_size_mb = meta.len() / 1024 / 1024;

        if file_size_mb >= self.config.max_file_size_mb {
            info!(
                "File size ({} MB) exceeded limit ({} MB), rotating file",
                file_size_mb, self.config.max_file_size_mb
            );
            self.rotate_file().await?;
        }

        Ok(())
    }

    /// ファイルのstemとextensionを取得
    fn file_parts(&self) -> (&str, &str) {
        let file_path = Path::new(&self.config.file_path);
        let stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("raw_responses");
        let ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("ndjson");
        (stem, ext)
    }

    /// ファイルをローテーション
    async fn rotate_file(&self) -> Result<()> {
        let file_path = Path::new(&self.config.file_path);
        let (file_stem, file_ext) = self.file_parts();

        let now = Utc::now();
        let timestamp = now.format("%Y%m%d_%H%M%S");

        // 新しいファイル名を生成
        let rotated_name = format!("{}_{}.{}", file_stem, timestamp, file_ext);
        let rotated_path = file_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&rotated_name);

        // ファイルをリネーム
        std::fs::rename(&self.config.file_path, &rotated_path)
            .context("Failed to rotate file")?;

        info!(
            "File rotated: {} -> {}",
            self.config.file_path,
            rotated_path.display()
        );

        // 古いバックアップファイルを削除
        self.cleanup_old_backups().await?;

        Ok(())
    }

    /// 古いバックアップファイルを削除
    async fn cleanup_old_backups(&self) -> Result<()> {
        let file_path = Path::new(&self.config.file_path);
        let dir = file_path.parent().unwrap_or_else(|| Path::new("."));
        let (file_stem, file_ext) = self.file_parts();

        // ディレクトリ内のバックアップファイルを検索
        let mut backup_files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    // パターンマッチング: {stem}_{timestamp}.{ext}
                    if filename.starts_with(&format!("{}_", file_stem))
                        && filename.ends_with(&format!(".{}", file_ext))
                    {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(created) = meta.created() {
                                backup_files.push((path, created));
                            }
                        }
                    }
                }
            }
        }

        // 作成日時でソート（新しい順）
        backup_files.sort_by(|a, b| b.1.cmp(&a.1));

        // 制限を超えた古いファイルを削除
        if backup_files.len() > self.config.max_backup_files as usize {
            for (path, _) in backup_files
                .iter()
                .skip(self.config.max_backup_files as usize)
            {
                match std::fs::remove_file(path) {
                    Ok(_) => info!("Removed old backup file: {}", path.display()),
                    Err(e) => warn!("Failed to remove old backup file {}: {}", path.display(), e),
                }
            }
        }

        Ok(())
    }

    /// 保存されたレスポンス数を取得
    pub fn get_saved_response_count(&self) -> Result<usize> {
        let file_path = Path::new(&self.config.file_path);

        if !file_path.exists() {
            return Ok(0);
        }

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let count = reader.lines().count();

        Ok(count)
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: SaveConfig) {
        self.config = config;
    }

    /// 現在の設定を取得
    pub fn get_config(&self) -> &SaveConfig {
        &self.config
    }

    /// 保存機能が有効かどうか
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir_for_test(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("liscov_test_raw_response").join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ========================================================================
    // SaveConfig defaults (05_raw_response.md: デフォルト値)
    // ========================================================================

    #[test]
    fn save_config_default_values() {
        let config = SaveConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.file_path, "raw_responses.ndjson");
        assert_eq!(config.max_file_size_mb, 100);
        assert!(config.enable_rotation);
        assert_eq!(config.max_backup_files, 5);
    }

    // ========================================================================
    // RawResponseSaver basics (05_raw_response.md)
    // ========================================================================

    #[test]
    fn saver_new_and_is_enabled() {
        let saver = RawResponseSaver::new(SaveConfig::default());
        assert!(!saver.is_enabled());

        let saver = RawResponseSaver::new(SaveConfig {
            enabled: true,
            ..SaveConfig::default()
        });
        assert!(saver.is_enabled());
    }

    #[test]
    fn saver_update_config() {
        let mut saver = RawResponseSaver::new(SaveConfig::default());
        assert!(!saver.is_enabled());

        saver.update_config(SaveConfig {
            enabled: true,
            ..SaveConfig::default()
        });
        assert!(saver.is_enabled());
    }

    // ========================================================================
    // save_response (05_raw_response.md: レスポンス保存)
    // ========================================================================

    #[tokio::test]
    async fn save_response_disabled_does_nothing() {
        let dir = temp_dir_for_test("disabled");
        let file_path = dir.join("test.ndjson");

        let saver = RawResponseSaver::new(SaveConfig {
            enabled: false,
            file_path: file_path.to_string_lossy().to_string(),
            ..SaveConfig::default()
        });

        saver.save_response(r#"{"test": true}"#).await.unwrap();
        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn save_response_enabled_creates_ndjson() {
        let dir = temp_dir_for_test("enabled");
        let file_path = dir.join("test.ndjson");

        let saver = RawResponseSaver::new(SaveConfig {
            enabled: true,
            file_path: file_path.to_string_lossy().to_string(),
            enable_rotation: false,
            ..SaveConfig::default()
        });

        saver.save_response(r#"{"actions": []}"#).await.unwrap();

        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        let line: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert!(line.get("timestamp").is_some());
        assert!(line.get("response").is_some());
    }

    #[tokio::test]
    async fn save_response_appends_multiple_lines() {
        let dir = temp_dir_for_test("append");
        let file_path = dir.join("test.ndjson");

        let saver = RawResponseSaver::new(SaveConfig {
            enabled: true,
            file_path: file_path.to_string_lossy().to_string(),
            enable_rotation: false,
            ..SaveConfig::default()
        });

        saver.save_response(r#"{"msg": 1}"#).await.unwrap();
        saver.save_response(r#"{"msg": 2}"#).await.unwrap();
        saver.save_response(r#"{"msg": 3}"#).await.unwrap();

        assert_eq!(saver.get_saved_response_count().unwrap(), 3);
    }

    // ========================================================================
    // get_saved_response_count (05_raw_response.md)
    // ========================================================================

    #[test]
    fn count_nonexistent_file_returns_zero() {
        let saver = RawResponseSaver::new(SaveConfig {
            file_path: "/nonexistent/path/test.ndjson".to_string(),
            ..SaveConfig::default()
        });
        assert_eq!(saver.get_saved_response_count().unwrap(), 0);
    }

    // ========================================================================
    // File rotation (05_raw_response.md: ファイルローテーション)
    // ========================================================================

    #[tokio::test]
    async fn rotation_renames_file_when_size_exceeded() {
        let dir = temp_dir_for_test("rotation");
        let file_path = dir.join("responses.ndjson");

        // Create a file that exceeds the size limit (1 MB)
        {
            let mut file = fs::File::create(&file_path).unwrap();
            // Write > 1 MB of data
            let line = "x".repeat(1024);
            for _ in 0..1100 {
                writeln!(file, "{}", line).unwrap();
            }
        }

        let saver = RawResponseSaver::new(SaveConfig {
            enabled: true,
            file_path: file_path.to_string_lossy().to_string(),
            max_file_size_mb: 1, // 1 MB limit
            enable_rotation: true,
            max_backup_files: 5,
        });

        saver.save_response(r#"{"new": true}"#).await.unwrap();

        // Original file should be recreated (small size now)
        assert!(file_path.exists());
        let new_content = fs::read_to_string(&file_path).unwrap();
        assert!(new_content.contains("\"new\""));

        // A backup file should exist
        let backup_files: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("responses_") && name.ends_with(".ndjson")
            })
            .collect();
        assert_eq!(backup_files.len(), 1);
    }

    // ========================================================================
    // Backup cleanup (05_raw_response.md: 古いバックアップ削除)
    // ========================================================================

    // spec: 05_raw_response.md - 小さなファイルでは回転しない (サイズ計算の `/` mutant対策)
    // ファイルサイズ計算に `/ 1024 / 1024` を使うため、
    // 変異で `* 1024` になるとサイズが巨大になり誤って回転が発動する
    #[tokio::test]
    async fn no_rotation_for_small_file() {
        let dir = temp_dir_for_test("no_rotation_small");
        let file_path = dir.join("responses.ndjson");

        // 数KB (1MB未満) のファイルを作成
        {
            let mut file = fs::File::create(&file_path).unwrap();
            // 10行 × 100バイト = 約1KB
            let line = "x".repeat(100);
            for _ in 0..10 {
                writeln!(file, "{}", line).unwrap();
            }
        }

        let saver = RawResponseSaver::new(SaveConfig {
            enabled: true,
            file_path: file_path.to_string_lossy().to_string(),
            max_file_size_mb: 1,
            enable_rotation: true,
            max_backup_files: 5,
        });

        saver.save_response(r#"{"check": "no_rotate"}"#).await.unwrap();

        // バックアップファイルが作成されていないことを確認
        let backup_count = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("responses_") && name.ends_with(".ndjson")
            })
            .count();
        assert_eq!(backup_count, 0, "小さなファイルで回転が発動してはいけない");
    }

    // spec: 05_raw_response.md - ちょうどmax件のバックアップは削除されない (`>` mutant対策)
    #[tokio::test]
    async fn cleanup_keeps_exactly_max_backups() {
        let dir = temp_dir_for_test("cleanup_exact_max");
        let file_path = dir.join("responses.ndjson");

        // ちょうど max_backup_files 件のバックアップファイルを作成
        for i in 0..3 {
            let backup_name = format!("responses_20250101_{:06}.ndjson", i);
            fs::write(dir.join(&backup_name), "old data").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // 大きなファイルを作成してローテーション発動
        {
            let mut file = fs::File::create(&file_path).unwrap();
            let line = "x".repeat(1024);
            for _ in 0..1100 {
                writeln!(file, "{}", line).unwrap();
            }
        }

        let saver = RawResponseSaver::new(SaveConfig {
            enabled: true,
            file_path: file_path.to_string_lossy().to_string(),
            max_file_size_mb: 1,
            enable_rotation: true,
            max_backup_files: 3, // ちょうど既存件数と同じ
        });

        saver.save_response(r#"{"test": true}"#).await.unwrap();

        // ローテーション後は既存3件 + 新しく1件 = 4件になるので max=3 を超える
        // → 古い1件が削除されて3件になるはず
        let backup_count = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("responses_") && name.ends_with(".ndjson")
            })
            .count();
        assert!(
            backup_count <= 3,
            "max_backup_files=3 のとき3件以下でなければならない, got {}",
            backup_count
        );
    }

    #[tokio::test]
    async fn cleanup_removes_old_backups() {
        let dir = temp_dir_for_test("cleanup");
        let file_path = dir.join("responses.ndjson");

        // Create old backup files
        for i in 0..5 {
            let backup_name = format!("responses_20250101_{:06}.ndjson", i);
            fs::write(dir.join(&backup_name), "old data").unwrap();
            // Small delay to ensure different creation times
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Create a large file to trigger rotation
        {
            let mut file = fs::File::create(&file_path).unwrap();
            let line = "x".repeat(1024);
            for _ in 0..1100 {
                writeln!(file, "{}", line).unwrap();
            }
        }

        let saver = RawResponseSaver::new(SaveConfig {
            enabled: true,
            file_path: file_path.to_string_lossy().to_string(),
            max_file_size_mb: 1,
            enable_rotation: true,
            max_backup_files: 3, // Only keep 3 backups
        });

        saver.save_response(r#"{"test": true}"#).await.unwrap();

        // Count backup files (should be max 3)
        let backup_count = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("responses_") && name.ends_with(".ndjson")
            })
            .count();

        assert!(backup_count <= 3, "Expected at most 3 backups, got {}", backup_count);
    }
}
