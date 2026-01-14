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

    /// ファイルをローテーション
    async fn rotate_file(&self) -> Result<()> {
        let file_path = Path::new(&self.config.file_path);
        let file_stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("raw_responses");
        let file_ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("ndjson");

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
        let file_stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("raw_responses");
        let file_ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("ndjson");

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
