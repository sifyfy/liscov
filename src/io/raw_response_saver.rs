//! YouTubeライブチャットレスポンスの保存とファイル管理
//!
//! このモジュールはYouTube APIからの生レスポンスをndjson形式で保存し、
//! ファイルサイズ制限やローテーション機能を提供します。

use crate::api::innertube::get_live_chat::{GetLiveChatResponse, ResponseEntry};
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

    /// レスポンスを保存
    pub async fn save_response(&self, response: &GetLiveChatResponse) -> Result<()> {
        tracing::info!(
            "💾 save_response called: enabled={}, file_path={}",
            self.config.enabled,
            self.config.file_path
        );

        if !self.config.enabled {
            tracing::info!("💾 Save response skipped: disabled");
            return Ok(());
        }

        tracing::info!(
            "💾 Starting raw response save process to: {}",
            self.config.file_path
        );

        // ファイルサイズチェックとローテーション
        if self.config.enable_rotation {
            self.check_and_rotate_file().await?;
        }

        // レスポンスエントリを作成
        let entry = ResponseEntry {
            timestamp: Utc::now().timestamp() as u64,
            response: response.clone(),
        };

        // JSONとして保存
        let json_line =
            serde_json::to_string(&entry).context("Failed to serialize response to JSON")?;

        tracing::info!("💾 JSON serialized, length: {} bytes", json_line.len());

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
        tracing::info!("💾 Opening file for append: {}", self.config.file_path);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.file_path)
            .context("Failed to open raw response file")?;

        tracing::info!(
            "💾 File opened successfully, writing {} bytes",
            json_line.len()
        );

        writeln!(file, "{}", json_line)?;
        file.flush()?;

        tracing::info!("💾 Data written and flushed to file successfully");
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
        let metadata = metadata(&self.config.file_path).await?;
        let file_size_mb = metadata.len() / 1024 / 1024;

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
        std::fs::rename(&self.config.file_path, &rotated_path).context("Failed to rotate file")?;

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
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(created) = metadata.created() {
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
    pub async fn get_saved_response_count(&self) -> Result<usize> {
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
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_save_config_default() {
        let config = SaveConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.file_path, "raw_responses.ndjson");
        assert_eq!(config.max_file_size_mb, 100);
        assert!(config.enable_rotation);
        assert_eq!(config.max_backup_files, 5);
    }

    #[tokio::test]
    async fn test_raw_response_saver_creation() {
        let config = SaveConfig::default();
        let saver = RawResponseSaver::new(config);
        assert!(!saver.is_enabled());
    }

    #[tokio::test]
    async fn test_save_response_disabled() {
        let config = SaveConfig::default(); // enabled = false
        let saver = RawResponseSaver::new(config);

        // ダミーレスポンス
        let response = GetLiveChatResponse {
            continuation_contents: crate::api::innertube::get_live_chat::ContinuationContents {
                live_chat_continuation:
                    crate::api::innertube::get_live_chat::LiveChatContinuation {
                        continuation: None,
                        actions: vec![],
                        continuations: vec![],
                    },
            },
        };

        // 無効化されているので保存されない
        assert!(saver.save_response(&response).await.is_ok());
    }

    #[tokio::test]
    async fn test_save_response_enabled() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_responses.ndjson");

        let mut config = SaveConfig::default();
        config.enabled = true;
        config.file_path = file_path.to_string_lossy().to_string();

        let saver = RawResponseSaver::new(config);

        // ダミーレスポンス
        let response = GetLiveChatResponse {
            continuation_contents: crate::api::innertube::get_live_chat::ContinuationContents {
                live_chat_continuation:
                    crate::api::innertube::get_live_chat::LiveChatContinuation {
                        continuation: Some(crate::api::innertube::get_live_chat::Continuation(
                            "test_token".to_string(),
                        )),
                        actions: vec![],
                        continuations: vec![],
                    },
            },
        };

        // 保存実行
        assert!(saver.save_response(&response).await.is_ok());

        // ファイルが作成されていることを確認
        assert!(file_path.exists());

        // レスポンス数を確認
        let count = saver.get_saved_response_count().await.unwrap();
        assert_eq!(count, 1);
    }
}
