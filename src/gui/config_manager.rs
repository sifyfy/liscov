//! アプリケーション設定管理モジュール
//!
//! XDGディレクトリを使用した設定ファイルの永続化と管理を提供します。

use crate::gui::models::AppState;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// ウィンドウ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub maximized: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 800,
            x: 100,
            y: 100,
            maximized: false,
        }
    }
}

/// ログ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// カスタムログディレクトリ（Noneの場合はXDGデフォルト使用）
    pub log_dir: Option<PathBuf>,
    /// ログレベル (trace/debug/info/warn/error)
    pub log_level: String,
    /// ファイル出力有効化
    pub enable_file_logging: bool,
    /// 保存するログファイル数上限
    pub max_log_files: u32,
    /// 古いログファイル自動削除
    pub auto_cleanup_enabled: bool,
    /// ログファイル名パターン（内部管理用）
    pub log_filename_pattern: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            log_dir: None,
            log_level: "info".to_string(),
            enable_file_logging: true,
            max_log_files: 30,
            auto_cleanup_enabled: true,
            log_filename_pattern: "liscov_*.log".to_string(),
        }
    }
}

/// アプリケーション設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// URL設定
    pub url: String,

    /// 自動保存設定
    pub auto_save_enabled: bool,
    pub output_file: String,

    /// 生レスポンス保存設定
    pub save_raw_responses: bool,
    pub raw_response_file: String,
    pub max_raw_file_size_mb: u64,
    pub enable_file_rotation: bool,

    /// アクティブタブ
    pub active_tab: String,

    /// ウィンドウ設定
    #[serde(default)]
    pub window: WindowConfig,

    /// ログ設定
    #[serde(default)]
    pub log: LogConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            url: "https://youtube.com/watch?v=".to_string(),
            auto_save_enabled: false, // デフォルトは無効
            output_file: "live_chat.ndjson".to_string(),
            save_raw_responses: true, // 生レスポンスはデフォルトで保存（デバッグ・検証用）
            raw_response_file: "raw_responses.ndjson".to_string(),
            max_raw_file_size_mb: 100,
            enable_file_rotation: true,
            active_tab: "ChatMonitor".to_string(),
            window: WindowConfig::default(),
            log: LogConfig::default(),
        }
    }
}

impl From<&AppState> for AppConfig {
    fn from(state: &AppState) -> Self {
        Self {
            // URLは設定として保存しない（起動時は常に空）
            url: String::new(),
            auto_save_enabled: state.auto_save_enabled,
            output_file: state.output_file.clone(),
            save_raw_responses: state.save_raw_responses,
            raw_response_file: state.raw_response_file.clone(),
            max_raw_file_size_mb: state.max_raw_file_size_mb,
            enable_file_rotation: state.enable_file_rotation,
            active_tab: format!("{:?}", state.active_tab),
            window: state.window.clone(),
            log: LogConfig::default(), // AppStateからは取得せず、デフォルト値を使用
        }
    }
}

/// 設定管理マネージャー
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    /// 新しい設定マネージャーを作成
    pub fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        // 設定ディレクトリを作成（存在しない場合）
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        Ok(Self { config_path })
    }

    /// XDGディレクトリに基づく設定ファイルパスを取得
    fn get_config_path() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("dev", "sifyfy", "liscov")
            .context("Failed to get project directories")?;

        let config_dir = project_dirs.config_dir();
        let config_file = config_dir.join("config.toml");

        debug!("Config file path: {}", config_file.display());

        Ok(config_file)
    }

    /// 設定を読み込み
    pub fn load_config(&self) -> Result<AppConfig> {
        if !self.config_path.exists() {
            info!(
                "Config file not found, using default settings: {}",
                self.config_path.display()
            );
            return Ok(AppConfig::default());
        }

        let config_content = fs::read_to_string(&self.config_path).with_context(|| {
            format!("Failed to read config file: {}", self.config_path.display())
        })?;

        let config: AppConfig = toml::from_str(&config_content).with_context(|| {
            format!(
                "Failed to parse config file: {}",
                self.config_path.display()
            )
        })?;

        info!(
            "✅ Configuration loaded from: {}",
            self.config_path.display()
        );

        Ok(config)
    }

    /// 設定を保存
    pub fn save_config(&self, config: &AppConfig) -> Result<()> {
        let config_content =
            toml::to_string_pretty(config).context("Failed to serialize config")?;

        fs::write(&self.config_path, config_content).with_context(|| {
            format!(
                "Failed to write config file: {}",
                self.config_path.display()
            )
        })?;

        info!("💾 Configuration saved to: {}", self.config_path.display());

        Ok(())
    }

    /// AppStateから設定を保存
    pub fn save_from_app_state(&self, state: &AppState) -> Result<()> {
        let config = AppConfig::from(state);
        self.save_config(&config)
    }

    /// 設定をAppStateに適用
    pub fn apply_to_app_state(&self, config: &AppConfig, state: &mut AppState) {
        state.url = config.url.clone();
        state.auto_save_enabled = config.auto_save_enabled;
        state.output_file = config.output_file.clone();
        state.save_raw_responses = config.save_raw_responses;
        state.raw_response_file = config.raw_response_file.clone();
        state.max_raw_file_size_mb = config.max_raw_file_size_mb;
        state.enable_file_rotation = config.enable_file_rotation;
        state.window = config.window.clone();

        // アクティブタブの復元
        state.active_tab = match config.active_tab.as_str() {
            "ChatMonitor" => crate::gui::models::ActiveTab::ChatMonitor,
            "RevenueAnalytics" => crate::gui::models::ActiveTab::RevenueAnalytics,
            "DataExport" => crate::gui::models::ActiveTab::DataExport,
            "Settings" => crate::gui::models::ActiveTab::Settings,
            _ => crate::gui::models::ActiveTab::ChatMonitor,
        };
    }

    /// 設定ファイルパスを取得（デバッグ用）
    pub fn get_config_file_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// 設定をリセット（デフォルト値に戻す）
    pub fn reset_config(&self) -> Result<()> {
        let default_config = AppConfig::default();
        self.save_config(&default_config)?;
        info!("🔄 Configuration reset to defaults");
        Ok(())
    }

    /// 設定ファイルが存在するかチェック
    pub fn config_exists(&self) -> bool {
        self.config_path.exists()
    }

    /// 設定ファイルをバックアップ
    pub fn backup_config(&self) -> Result<PathBuf> {
        if !self.config_path.exists() {
            return Err(anyhow::anyhow!("Config file does not exist"));
        }

        let backup_path = self.config_path.with_extension("toml.bak");
        fs::copy(&self.config_path, &backup_path)
            .with_context(|| format!("Failed to backup config to: {}", backup_path.display()))?;

        info!("📋 Configuration backed up to: {}", backup_path.display());

        Ok(backup_path)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ConfigManager")
    }
}

/// グローバル設定マネージャーインスタンス
static CONFIG_MANAGER: std::sync::OnceLock<std::sync::Mutex<ConfigManager>> =
    std::sync::OnceLock::new();

/// グローバル設定マネージャーを取得
pub fn get_config_manager() -> &'static std::sync::Mutex<ConfigManager> {
    CONFIG_MANAGER.get_or_init(|| {
        debug!("🏗️ Creating global config manager");
        match ConfigManager::new() {
            Ok(manager) => std::sync::Mutex::new(manager),
            Err(e) => {
                warn!("❌ Failed to create config manager, using default: {}", e);
                // フォールバック用の基本的なパスを使用
                let fallback_path = std::env::current_dir()
                    .unwrap_or_default()
                    .join("liscov_config.toml");
                std::sync::Mutex::new(ConfigManager {
                    config_path: fallback_path,
                })
            }
        }
    })
}

/// 設定を非同期で保存（GUI用）
pub fn save_config_async(config: AppConfig) {
    tokio::spawn(async move {
        let manager = get_config_manager();
        if let Ok(manager_guard) = manager.lock() {
            if let Err(e) = manager_guard.save_config(&config) {
                warn!("❌ Failed to save config: {}", e);
            }
        }
    });
}

/// AppStateから設定を非同期で保存（GUI用）
pub fn save_app_state_async(state: AppState) {
    tokio::spawn(async move {
        let config = AppConfig::from(&state);
        let manager = get_config_manager();
        if let Ok(manager_guard) = manager.lock() {
            if let Err(e) = manager_guard.save_config(&config) {
                warn!("❌ Failed to save app state config: {}", e);
            }
        }
    });
}

/// 現在の設定をグローバルに取得（サービス側で使用）
pub fn get_current_config() -> Option<AppConfig> {
    let manager = get_config_manager();
    if let Ok(manager_guard) = manager.lock() {
        manager_guard.load_config().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();

        assert_eq!(config.url, deserialized.url);
        assert_eq!(config.auto_save_enabled, deserialized.auto_save_enabled);
    }

    #[test]
    fn test_config_manager_save_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let manager = ConfigManager { config_path };
        let original_config = AppConfig {
            auto_save_enabled: true,
            url: "https://example.com".to_string(),
            ..AppConfig::default()
        };

        // 保存
        manager.save_config(&original_config).unwrap();

        // 読み込み
        let loaded_config = manager.load_config().unwrap();

        assert_eq!(
            original_config.auto_save_enabled,
            loaded_config.auto_save_enabled
        );
        assert_eq!(original_config.url, loaded_config.url);
    }

    #[test]
    fn test_config_load_nonexistent_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.toml");

        let manager = ConfigManager { config_path };

        // 存在しないファイルの読み込み時はデフォルトが返される
        let loaded_config = manager.load_config().unwrap();
        let default_config = AppConfig::default();

        assert_eq!(loaded_config.url, default_config.url);
        assert_eq!(
            loaded_config.auto_save_enabled,
            default_config.auto_save_enabled
        );
    }

    #[test]
    fn test_config_load_corrupted_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("corrupted.toml");

        // 破損したTOMLファイルを作成
        std::fs::write(&config_path, "invalid toml content [unclosed section").unwrap();

        let manager = ConfigManager { config_path };

        // 破損したファイルの場合はエラーが返される
        let result = manager.load_config();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse config file"));
    }

    #[test]
    fn test_config_load_partial_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("partial.toml");

        // 部分的なTOMLファイルを作成（すべての必須フィールドを含む）
        let partial_toml = r#"
url = "https://partial.example.com"
auto_save_enabled = false
output_file = "test.ndjson"
save_raw_responses = false
raw_response_file = "raw.ndjson"
max_raw_file_size_mb = 50
enable_file_rotation = true
active_tab = "ChatMonitor"
"#;
        std::fs::write(&config_path, partial_toml).unwrap();

        let manager = ConfigManager { config_path };
        let loaded_config = manager.load_config().unwrap();

        // 指定されたフィールドは読み込まれ、省略されたフィールドはデフォルト値になる
        assert_eq!(loaded_config.url, "https://partial.example.com");
        assert_eq!(loaded_config.auto_save_enabled, false);
        assert_eq!(loaded_config.max_raw_file_size_mb, 50);
    }

    #[test]
    #[cfg(unix)]
    fn test_config_save_to_readonly_directory() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempdir().unwrap();
        let readonly_dir = temp_dir.path().join("readonly");
        fs::create_dir(&readonly_dir).unwrap();

        // ディレクトリを読み取り専用に設定
        let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
        perms.set_mode(0o444); // 読み取り専用
        fs::set_permissions(&readonly_dir, perms).unwrap();

        let config_path = readonly_dir.join("config.toml");
        let manager = ConfigManager { config_path };
        let config = AppConfig::default();

        // 読み取り専用ディレクトリへの保存は失敗する
        let result = manager.save_config(&config);
        assert!(result.is_err());

        // 権限を元に戻してクリーンアップ
        let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&readonly_dir, perms).unwrap();
    }

    #[test]
    fn test_config_save_invalid_path() {
        // 無効なパス（存在しないディレクトリ）
        let config_path = PathBuf::from("/nonexistent/directory/config.toml");
        let manager = ConfigManager { config_path };
        let config = AppConfig::default();

        // 存在しないディレクトリへの保存は失敗する
        let result = manager.save_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_with_special_characters() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("special_chars.toml");

        let manager = ConfigManager { config_path };
        let special_config = AppConfig {
            url: "https://example.com/path?param=value&other=\"quoted\"".to_string(),
            auto_save_enabled: true,
            ..AppConfig::default()
        };

        // 特殊文字を含むURLの保存と読み込み
        manager.save_config(&special_config).unwrap();
        let loaded_config = manager.load_config().unwrap();

        assert_eq!(special_config.url, loaded_config.url);
    }

    #[test]
    fn test_config_unicode_support() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("unicode.toml");

        let manager = ConfigManager { config_path };
        let unicode_config = AppConfig {
            url: "https://例え.テスト/パス?パラメータ=値&🔥=🚀".to_string(),
            auto_save_enabled: true,
            ..AppConfig::default()
        };

        // Unicode文字の保存と読み込み
        manager.save_config(&unicode_config).unwrap();
        let loaded_config = manager.load_config().unwrap();

        assert_eq!(unicode_config.url, loaded_config.url);
    }

    #[test]
    fn test_config_extreme_values() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("extreme.toml");

        let manager = ConfigManager { config_path };
        let extreme_config = AppConfig {
            url: "x".repeat(10000), // 非常に長いURL
            auto_save_enabled: true,
            ..AppConfig::default()
        };

        // 極端に長い値の保存と読み込み
        manager.save_config(&extreme_config).unwrap();
        let loaded_config = manager.load_config().unwrap();

        assert_eq!(extreme_config.url, loaded_config.url);
        assert_eq!(extreme_config.url.len(), 10000);
    }

    #[test]
    fn test_config_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("concurrent.toml");
        let manager = Arc::new(ConfigManager { config_path });

        let mut handles = Vec::new();

        // 複数のスレッドから同時に設定を保存・読み込み
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                let config = AppConfig {
                    url: format!("https://thread{}.example.com", i),
                    auto_save_enabled: i % 2 == 0,
                    ..AppConfig::default()
                };

                // 保存と読み込みを繰り返す
                for _ in 0..10 {
                    if let Err(_) = manager_clone.save_config(&config) {
                        // 並行アクセスでファイルロックに失敗する場合がある
                        continue;
                    }
                    if let Ok(loaded) = manager_clone.load_config() {
                        // 最後に保存されたいずれかの設定が読み込まれる
                        assert!(loaded.url.starts_with("https://"));
                    }
                }
            });
            handles.push(handle);
        }

        // すべてのスレッドの完了を待つ
        for handle in handles {
            handle.join().unwrap();
        }

        // 最終的に有効な設定が保存されているかを確認（エラーを許容）
        if let Ok(final_config) = manager.load_config() {
            assert!(final_config.url.starts_with("https://"));
        }
    }

    #[test]
    fn test_config_file_recovery() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("recovery.toml");

        let manager = ConfigManager {
            config_path: config_path.clone(),
        };

        // 正常な設定を保存
        let valid_config = AppConfig {
            url: "https://valid.example.com".to_string(),
            auto_save_enabled: true,
            ..AppConfig::default()
        };
        manager.save_config(&valid_config).unwrap();

        // ファイルを破損させる
        std::fs::write(&config_path, "broken toml content").unwrap();

        // 破損したファイルからの読み込み時はエラーが返される
        let result = manager.load_config();
        assert!(result.is_err());

        // 再度正常な設定を保存してリカバリ
        manager.save_config(&valid_config).unwrap();
        let final_config = manager.load_config().unwrap();

        assert_eq!(final_config.url, valid_config.url);
    }

    #[test]
    fn test_config_backup_and_restore() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("backup_test.toml");
        let backup_path = temp_dir.path().join("backup_test.toml.backup");

        let manager = ConfigManager {
            config_path: config_path.clone(),
        };

        let original_config = AppConfig {
            url: "https://original.example.com".to_string(),
            auto_save_enabled: false,
            ..AppConfig::default()
        };

        // 元の設定を保存
        manager.save_config(&original_config).unwrap();

        // バックアップを作成
        std::fs::copy(&config_path, &backup_path).unwrap();

        // 設定を変更
        let modified_config = AppConfig {
            url: "https://modified.example.com".to_string(),
            auto_save_enabled: true,
            ..AppConfig::default()
        };
        manager.save_config(&modified_config).unwrap();

        // バックアップから復元
        std::fs::copy(&backup_path, &config_path).unwrap();
        let restored_config = manager.load_config().unwrap();

        assert_eq!(restored_config.url, original_config.url);
        assert_eq!(
            restored_config.auto_save_enabled,
            original_config.auto_save_enabled
        );
    }
}
