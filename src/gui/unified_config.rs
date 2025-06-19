//! 統一設定管理システム
//!
//! Phase 2実装: 設定管理統一

use async_trait::async_trait;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::traits::{ConfigError, ConfigurationManager};
use crate::io::SaveConfig;

/// ハイライト設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightConfig {
    /// ハイライト機能の有効/無効
    pub enabled: bool,
    /// ハイライト表示時間（秒）
    pub duration_seconds: u64,
    /// 同時ハイライト最大メッセージ数
    pub max_messages: usize,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            enabled: true, // デフォルトは有効
            duration_seconds: 8,
            max_messages: 20,
        }
    }
}

impl HighlightConfig {
    /// 補完ハイライトの最大数を自動計算（メイン設定の50%切り上げ）
    pub fn get_backup_max_messages(&self) -> usize {
        ((self.max_messages as f32) * 0.5).ceil() as usize
    }

    /// 補完チェック間隔を取得（固定値500ms）
    pub fn get_backup_check_interval_ms(&self) -> u64 {
        500
    }
}

/// 統一設定管理マネージャー
pub struct UnifiedConfigManager {
    /// 設定ファイルのベースディレクトリ
    config_dir: PathBuf,
    /// メモリキャッシュ（読み書き性能向上のため）
    cache: RwLock<HashMap<String, serde_json::Value>>,
    /// 変更監視フラグ
    dirty_keys: RwLock<HashMap<String, bool>>,
}

impl UnifiedConfigManager {
    /// 新しい統一設定マネージャーを作成
    pub async fn new() -> Result<Self, ConfigError> {
        let config_dir = Self::get_config_directory()?;

        // 設定ディレクトリを作成
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).await.map_err(|e| {
                ConfigError::FileAccess(format!("Failed to create config directory: {}", e))
            })?;
        }

        let manager = Self {
            config_dir,
            cache: RwLock::new(HashMap::new()),
            dirty_keys: RwLock::new(HashMap::new()),
        };

        // 既存の設定をロード
        manager.load_all_configs().await?;

        debug!(
            "✅ Unified config manager initialized: {}",
            manager.config_dir.display()
        );
        Ok(manager)
    }

    /// XDGディレクトリに基づく設定ディレクトリを取得
    fn get_config_directory() -> Result<PathBuf, ConfigError> {
        let project_dirs = ProjectDirs::from("dev", "sifyfy", "liscov").ok_or_else(|| {
            ConfigError::FileAccess("Failed to get project directories".to_string())
        })?;

        Ok(project_dirs.config_dir().to_path_buf())
    }

    /// 設定ファイルパスを取得
    fn get_config_file_path(&self, category: &str) -> PathBuf {
        self.config_dir.join(format!("{}.toml", category))
    }

    /// すべての設定をロード
    async fn load_all_configs(&self) -> Result<(), ConfigError> {
        if !self.config_dir.exists() {
            return Ok(());
        }

        let mut dir_entries = fs::read_dir(&self.config_dir).await.map_err(|e| {
            ConfigError::FileAccess(format!("Failed to read config directory: {}", e))
        })?;

        let mut cache = self.cache.write().await;

        while let Some(entry) = dir_entries.next_entry().await.map_err(|e| {
            ConfigError::FileAccess(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(category) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_category_config(category).await {
                        Ok(config) => {
                            cache.insert(category.to_string(), config);
                            debug!("📁 Loaded config category: {}", category);
                        }
                        Err(e) => {
                            warn!("⚠️ Failed to load config category '{}': {}", category, e);
                        }
                    }
                }
            }
        }

        debug!("📋 Loaded {} config categories", cache.len());
        Ok(())
    }

    /// 特定カテゴリの設定をロード
    async fn load_category_config(&self, category: &str) -> Result<serde_json::Value, ConfigError> {
        let file_path = self.get_config_file_path(category);

        if !file_path.exists() {
            return Ok(serde_json::Value::Object(serde_json::Map::new()));
        }

        let content = fs::read_to_string(&file_path).await.map_err(|e| {
            ConfigError::FileAccess(format!(
                "Failed to read config file '{}': {}",
                file_path.display(),
                e
            ))
        })?;

        let toml_value: toml::Value = toml::from_str(&content).map_err(|e| {
            ConfigError::Serialization(format!("Failed to parse TOML '{}': {}", category, e))
        })?;

        let json_value = self.toml_to_json(toml_value)?;
        Ok(json_value)
    }

    /// 特定カテゴリの設定を保存
    async fn save_category_config(
        &self,
        category: &str,
        config: &serde_json::Value,
    ) -> Result<(), ConfigError> {
        let file_path = self.get_config_file_path(category);

        let toml_value = self.json_to_toml(config.clone())?;
        let content = toml::to_string_pretty(&toml_value).map_err(|e| {
            ConfigError::Serialization(format!("Failed to serialize TOML '{}': {}", category, e))
        })?;

        fs::write(&file_path, content).await.map_err(|e| {
            ConfigError::FileAccess(format!(
                "Failed to write config file '{}': {}",
                file_path.display(),
                e
            ))
        })?;

        debug!("💾 Saved config category: {}", category);
        Ok(())
    }

    /// TOMLをJSONに変換
    fn toml_to_json(&self, toml_value: toml::Value) -> Result<serde_json::Value, ConfigError> {
        let json_str = serde_json::to_string(&toml_value).map_err(|e| {
            ConfigError::Serialization(format!("TOML to JSON conversion failed: {}", e))
        })?;

        serde_json::from_str(&json_str)
            .map_err(|e| ConfigError::Serialization(format!("JSON parsing failed: {}", e)))
    }

    /// JSONをTOMLに変換
    fn json_to_toml(&self, json_value: serde_json::Value) -> Result<toml::Value, ConfigError> {
        let json_str = serde_json::to_string(&json_value)
            .map_err(|e| ConfigError::Serialization(format!("JSON serialization failed: {}", e)))?;

        serde_json::from_str::<toml::Value>(&json_str).map_err(|e| {
            ConfigError::Serialization(format!("JSON to TOML conversion failed: {}", e))
        })
    }

    /// 設定キーをカテゴリと名前に分割
    fn parse_config_key(&self, key: &str) -> (String, String) {
        if let Some(dot_pos) = key.find('.') {
            let category = key[..dot_pos].to_string();
            let name = key[dot_pos + 1..].to_string();
            (category, name)
        } else {
            ("app".to_string(), key.to_string())
        }
    }

    /// 変更されたカテゴリを保存
    pub async fn flush_dirty_configs(&self) -> Result<(), ConfigError> {
        let dirty_keys = self.dirty_keys.read().await;
        let cache = self.cache.read().await;

        let mut categories_to_save: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for (key, is_dirty) in dirty_keys.iter() {
            if *is_dirty {
                let (category, _) = self.parse_config_key(key);
                categories_to_save.insert(category);
            }
        }

        drop(dirty_keys);
        drop(cache);

        for category in categories_to_save {
            let cache = self.cache.read().await;
            if let Some(config) = cache.get(&category) {
                self.save_category_config(&category, config).await?;
            }
        }

        let mut dirty_keys = self.dirty_keys.write().await;
        dirty_keys.clear();

        Ok(())
    }

    /// 型安全な設定アクセスのヘルパー
    pub async fn get_typed_config<T>(&self, key: &str) -> Result<Option<T>, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        super::traits::ConfigurationHelper::get_typed_config(self, key).await
    }

    /// 型安全な設定保存のヘルパー
    pub async fn set_typed_config<T>(&self, key: &str, value: &T) -> Result<(), ConfigError>
    where
        T: Serialize + Send + Sync,
    {
        super::traits::ConfigurationHelper::set_typed_config(self, key, value).await
    }

    /// アプリケーション設定の移行（既存ConfigManagerとの互換性）
    pub async fn migrate_from_legacy(
        &self,
        legacy_config: &super::config_manager::AppConfig,
    ) -> Result<(), ConfigError> {
        // アプリケーション設定を移行
        self.set_typed_config("app.url", &legacy_config.url).await?;
        self.set_typed_config("app.auto_save_enabled", &legacy_config.auto_save_enabled)
            .await?;
        self.set_typed_config("app.output_file", &legacy_config.output_file)
            .await?;
        self.set_typed_config("app.active_tab", &legacy_config.active_tab)
            .await?;

        // ウィンドウ設定を移行
        self.set_typed_config("window.width", &legacy_config.window.width)
            .await?;
        self.set_typed_config("window.height", &legacy_config.window.height)
            .await?;
        self.set_typed_config("window.x", &legacy_config.window.x)
            .await?;
        self.set_typed_config("window.y", &legacy_config.window.y)
            .await?;
        self.set_typed_config("window.maximized", &legacy_config.window.maximized)
            .await?;

        // 生レスポンス設定を移行
        let save_config = SaveConfig {
            enabled: legacy_config.save_raw_responses,
            file_path: legacy_config.raw_response_file.clone(),
            max_file_size_mb: legacy_config.max_raw_file_size_mb,
            enable_rotation: legacy_config.enable_file_rotation,
            max_backup_files: 5, // デフォルト値
        };
        self.set_typed_config("raw_response", &save_config).await?;

        self.flush_dirty_configs().await?;

        info!("🔄 Legacy config migrated to unified system");
        Ok(())
    }

    /// レガシー形式への変換（互換性維持）
    pub async fn to_legacy_config(&self) -> Result<super::config_manager::AppConfig, ConfigError> {
        let url: String = self
            .get_typed_config("app.url")
            .await?
            .unwrap_or_else(|| "https://youtube.com/watch?v=".to_string());
        let auto_save_enabled: bool = self
            .get_typed_config("app.auto_save_enabled")
            .await?
            .unwrap_or(false);
        let output_file: String = self
            .get_typed_config("app.output_file")
            .await?
            .unwrap_or_else(|| "live_chat.ndjson".to_string());
        let active_tab: String = self
            .get_typed_config("app.active_tab")
            .await?
            .unwrap_or_else(|| "ChatMonitor".to_string());

        let window = super::config_manager::WindowConfig {
            width: self.get_typed_config("window.width").await?.unwrap_or(1200),
            height: self.get_typed_config("window.height").await?.unwrap_or(800),
            x: self.get_typed_config("window.x").await?.unwrap_or(100),
            y: self.get_typed_config("window.y").await?.unwrap_or(100),
            maximized: self
                .get_typed_config("window.maximized")
                .await?
                .unwrap_or(false),
        };

        let save_config: Option<SaveConfig> = self.get_typed_config("raw_response").await?;
        let save_config = save_config.unwrap_or_default();

        Ok(super::config_manager::AppConfig {
            url,
            auto_save_enabled,
            output_file,
            save_raw_responses: save_config.enabled,
            raw_response_file: save_config.file_path,
            max_raw_file_size_mb: save_config.max_file_size_mb,
            enable_file_rotation: save_config.enable_rotation,
            active_tab,
            window,
            log: super::config_manager::LogConfig::default(),
        })
    }
}

#[async_trait]
impl ConfigurationManager for UnifiedConfigManager {
    async fn get_config_json(&self, key: &str) -> Result<Option<serde_json::Value>, ConfigError> {
        let (category, name) = self.parse_config_key(key);

        let cache = self.cache.read().await;

        if let Some(category_config) = cache.get(&category) {
            if let Some(object) = category_config.as_object() {
                Ok(object.get(&name).cloned())
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn set_config_json(
        &self,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), ConfigError> {
        let (category, name) = self.parse_config_key(key);

        let mut cache = self.cache.write().await;

        let category_config = cache
            .entry(category.clone())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

        if let Some(object) = category_config.as_object_mut() {
            object.insert(name, value.clone());
        }

        drop(cache);

        // 変更をマーク
        let mut dirty_keys = self.dirty_keys.write().await;
        dirty_keys.insert(key.to_string(), true);

        debug!("📝 Config updated: {} = {:?}", key, value);
        Ok(())
    }

    async fn remove_config(&self, key: &str) -> Result<(), ConfigError> {
        let (category, name) = self.parse_config_key(key);

        let mut cache = self.cache.write().await;

        if let Some(category_config) = cache.get_mut(&category) {
            if let Some(object) = category_config.as_object_mut() {
                object.remove(&name);
            }
        }

        drop(cache);

        // 変更をマーク
        let mut dirty_keys = self.dirty_keys.write().await;
        dirty_keys.insert(key.to_string(), true);

        debug!("🗑️ Config removed: {}", key);
        Ok(())
    }

    async fn get_all_configs(&self) -> Result<HashMap<String, serde_json::Value>, ConfigError> {
        let cache = self.cache.read().await;
        let mut result = HashMap::new();

        for (category, config) in cache.iter() {
            if let Some(object) = config.as_object() {
                for (name, value) in object.iter() {
                    let full_key = format!("{}.{}", category, name);
                    result.insert(full_key, value.clone());
                }
            }
        }

        Ok(result)
    }

    fn validate_config_json(&self, value: &serde_json::Value) -> Result<(), ConfigError> {
        // 基本的なJSONバリデーション
        match value {
            serde_json::Value::Null => Ok(()),
            serde_json::Value::Bool(_) => Ok(()),
            serde_json::Value::Number(n) => {
                // f64に変換して有限数かチェック
                if let Some(f) = n.as_f64() {
                    if f.is_finite() {
                        Ok(())
                    } else {
                        Err(ConfigError::Validation(
                            "Infinite or NaN number not allowed".to_string(),
                        ))
                    }
                } else {
                    // i64やu64の場合は常に有限
                    Ok(())
                }
            }
            serde_json::Value::String(s) => {
                if s.len() > 10000 {
                    Err(ConfigError::Validation(
                        "String too long (max 10000 chars)".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
            serde_json::Value::Array(arr) => {
                if arr.len() > 1000 {
                    Err(ConfigError::Validation(
                        "Array too large (max 1000 elements)".to_string(),
                    ))
                } else {
                    // 再帰的に配列要素を検証
                    for item in arr {
                        self.validate_config_json(item)?;
                    }
                    Ok(())
                }
            }
            serde_json::Value::Object(obj) => {
                if obj.len() > 100 {
                    Err(ConfigError::Validation(
                        "Object too large (max 100 keys)".to_string(),
                    ))
                } else {
                    // 再帰的にオブジェクト値を検証
                    for value in obj.values() {
                        self.validate_config_json(value)?;
                    }
                    Ok(())
                }
            }
        }
    }
}

/// ファクトリ関数
impl UnifiedConfigManager {
    /// デフォルト設定でマネージャーを作成
    pub async fn create_default() -> Result<Self, ConfigError> {
        Self::new().await
    }

    /// 既存の設定ディレクトリからマネージャーを作成
    pub async fn create_from_directory(config_dir: PathBuf) -> Result<Self, ConfigError> {
        let manager = Self {
            config_dir,
            cache: RwLock::new(HashMap::new()),
            dirty_keys: RwLock::new(HashMap::new()),
        };

        manager.load_all_configs().await?;
        Ok(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_unified_config_creation() {
        let temp_dir = tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let manager = UnifiedConfigManager::create_from_directory(config_dir)
            .await
            .unwrap();

        // 基本的な設定操作をテスト
        manager
            .set_typed_config("test.value", &42i32)
            .await
            .unwrap();
        let value: Option<i32> = manager.get_typed_config("test.value").await.unwrap();

        assert_eq!(value, Some(42));
    }

    #[tokio::test]
    async fn test_config_categories() {
        let temp_dir = tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let manager = UnifiedConfigManager::create_from_directory(config_dir)
            .await
            .unwrap();

        // 複数のカテゴリに設定を保存
        manager
            .set_typed_config("app.name", &"liscov".to_string())
            .await
            .unwrap();
        manager
            .set_typed_config("window.width", &800u32)
            .await
            .unwrap();
        manager
            .set_typed_config("debug.enabled", &true)
            .await
            .unwrap();

        // 設定をフラッシュ
        manager.flush_dirty_configs().await.unwrap();

        // 値を確認
        let app_name: Option<String> = manager.get_typed_config("app.name").await.unwrap();
        let window_width: Option<u32> = manager.get_typed_config("window.width").await.unwrap();
        let debug_enabled: Option<bool> = manager.get_typed_config("debug.enabled").await.unwrap();

        assert_eq!(app_name, Some("liscov".to_string()));
        assert_eq!(window_width, Some(800));
        assert_eq!(debug_enabled, Some(true));
    }

    #[tokio::test]
    async fn test_config_validation() {
        let temp_dir = tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let manager = UnifiedConfigManager::create_from_directory(config_dir)
            .await
            .unwrap();

        // 有効な値
        let valid_value = serde_json::json!({"key": "value"});
        assert!(manager.validate_config_json(&valid_value).is_ok());

        // 無効な値（文字列が長すぎる）
        let invalid_value = serde_json::json!({"key": "x".repeat(20000)});
        assert!(manager.validate_config_json(&invalid_value).is_err());
    }

    #[tokio::test]
    async fn test_legacy_migration() {
        let temp_dir = tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let manager = UnifiedConfigManager::create_from_directory(config_dir)
            .await
            .unwrap();

        // レガシー設定を作成
        let legacy_config = super::super::config_manager::AppConfig {
            url: "https://example.com".to_string(),
            auto_save_enabled: true,
            output_file: "test.ndjson".to_string(),
            ..Default::default()
        };

        // 移行を実行
        manager.migrate_from_legacy(&legacy_config).await.unwrap();

        // 移行された値を確認
        let url: Option<String> = manager.get_typed_config("app.url").await.unwrap();
        let auto_save: Option<bool> = manager
            .get_typed_config("app.auto_save_enabled")
            .await
            .unwrap();

        assert_eq!(url, Some("https://example.com".to_string()));
        assert_eq!(auto_save, Some(true));

        // レガシー形式への変換をテスト
        let converted = manager.to_legacy_config().await.unwrap();
        assert_eq!(converted.url, "https://example.com");
        assert_eq!(converted.auto_save_enabled, true);
    }
}
