//! Cookie管理モジュール
//!
//! YouTube認証に必要なCookieの保存・読み込みを管理します。

use super::{AuthError, AuthResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// YouTube認証に必要なCookie
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YouTubeCookies {
    /// セッションID
    #[serde(default)]
    pub sid: String,
    /// HTTPセキュアセッションID
    #[serde(default)]
    pub hsid: String,
    /// セキュアセッションID
    #[serde(default)]
    pub ssid: String,
    /// API用セッションID
    #[serde(default)]
    pub apisid: String,
    /// セキュアAPI用セッションID（SAPISIDHASH生成に使用）
    #[serde(default)]
    pub sapisid: String,
    /// Cookie取得日時
    #[serde(default = "Utc::now")]
    pub acquired_at: DateTime<Utc>,
    /// 生のCookie文字列（DevToolsからコピーした全Cookie）
    /// この値が設定されている場合、個別のCookie値より優先される
    #[serde(default)]
    pub raw_cookies: Option<String>,
}

impl YouTubeCookies {
    /// 新しいYouTubeCookiesを作成
    pub fn new(sid: String, hsid: String, ssid: String, apisid: String, sapisid: String) -> Self {
        Self {
            sid,
            hsid,
            ssid,
            apisid,
            sapisid,
            acquired_at: Utc::now(),
            raw_cookies: None,
        }
    }

    /// 生のCookie文字列から作成
    pub fn from_raw(raw_cookies: String) -> Self {
        // SAPISIDを抽出（SAPISIDHASH生成に必要）
        let sapisid = Self::extract_cookie_value(&raw_cookies, "SAPISID")
            .unwrap_or_default();

        Self {
            sid: String::new(),
            hsid: String::new(),
            ssid: String::new(),
            apisid: String::new(),
            sapisid,
            acquired_at: Utc::now(),
            raw_cookies: Some(raw_cookies),
        }
    }

    /// Cookie文字列から特定のCookie値を抽出
    fn extract_cookie_value(cookies: &str, name: &str) -> Option<String> {
        for part in cookies.split(';') {
            let part = part.trim();
            if let Some(pos) = part.find('=') {
                let key = &part[..pos];
                if key == name {
                    return Some(part[pos + 1..].to_string());
                }
            }
        }
        None
    }

    /// すべてのCookieが空でないか確認
    pub fn is_valid(&self) -> bool {
        // raw_cookiesが設定されている場合はそれをチェック
        if let Some(ref raw) = self.raw_cookies {
            return !raw.is_empty() && raw.contains("SAPISID=");
        }
        // 個別Cookie値をチェック
        !self.sid.is_empty()
            && !self.hsid.is_empty()
            && !self.ssid.is_empty()
            && !self.apisid.is_empty()
            && !self.sapisid.is_empty()
    }

    /// HTTPリクエスト用のCookieヘッダー文字列を生成
    pub fn to_cookie_header(&self) -> String {
        // raw_cookiesが設定されている場合はそれを使用
        if let Some(ref raw) = self.raw_cookies {
            return raw.clone();
        }
        // 個別Cookie値から生成
        format!(
            "SID={}; HSID={}; SSID={}; APISID={}; SAPISID={}",
            self.sid, self.hsid, self.ssid, self.apisid, self.sapisid
        )
    }
}

/// Cookie設定ファイルの構造
#[derive(Debug, Serialize, Deserialize)]
struct CookieConfig {
    youtube: YouTubeCookies,
}

/// Cookie管理
pub struct CookieManager {
    /// 設定ファイルのパス
    config_path: PathBuf,
}

impl CookieManager {
    /// 新しいCookieManagerを作成
    ///
    /// # Arguments
    ///
    /// * `config_dir` - 設定ディレクトリのパス（例: ~/.config/liscov）
    pub fn new(config_dir: PathBuf) -> Self {
        let config_path = config_dir.join("credentials.toml");
        Self { config_path }
    }

    /// デフォルトの設定ディレクトリを使用してCookieManagerを作成
    pub fn with_default_dir() -> AuthResult<Self> {
        let config_dir = directories::ProjectDirs::from("dev", "sifyfy", "liscov")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .ok_or_else(|| AuthError::LoadError("Failed to determine config directory".into()))?;

        Ok(Self::new(config_dir))
    }

    /// Cookieを保存
    pub fn save(&self, cookies: &YouTubeCookies) -> AuthResult<()> {
        // ディレクトリが存在しない場合は作成
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let config = CookieConfig {
            youtube: cookies.clone(),
        };

        let toml_string = toml::to_string_pretty(&config)?;
        fs::write(&self.config_path, toml_string)?;

        Ok(())
    }

    /// Cookieを読み込み
    pub fn load(&self) -> AuthResult<YouTubeCookies> {
        if !self.config_path.exists() {
            return Err(AuthError::LoadError("Credentials file not found".into()));
        }

        let content = fs::read_to_string(&self.config_path)?;
        let config: CookieConfig = toml::from_str(&content)?;

        if !config.youtube.is_valid() {
            return Err(AuthError::LoadError("Invalid or incomplete cookies".into()));
        }

        Ok(config.youtube)
    }

    /// Cookieが存在するか確認
    pub fn exists(&self) -> bool {
        self.config_path.exists()
    }

    /// Cookieを削除
    pub fn delete(&self) -> AuthResult<()> {
        if self.config_path.exists() {
            fs::remove_file(&self.config_path)?;
        }
        Ok(())
    }

    /// 設定ファイルのパスを取得
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_cookies() -> YouTubeCookies {
        YouTubeCookies::new(
            "test_sid".into(),
            "test_hsid".into(),
            "test_ssid".into(),
            "test_apisid".into(),
            "test_sapisid".into(),
        )
    }

    #[test]
    fn test_youtube_cookies_new() {
        let cookies = create_test_cookies();

        assert_eq!(cookies.sid, "test_sid");
        assert_eq!(cookies.hsid, "test_hsid");
        assert_eq!(cookies.ssid, "test_ssid");
        assert_eq!(cookies.apisid, "test_apisid");
        assert_eq!(cookies.sapisid, "test_sapisid");
    }

    #[test]
    fn test_youtube_cookies_is_valid() {
        let valid_cookies = create_test_cookies();
        assert!(valid_cookies.is_valid());

        let invalid_cookies = YouTubeCookies::new(
            "".into(),
            "test_hsid".into(),
            "test_ssid".into(),
            "test_apisid".into(),
            "test_sapisid".into(),
        );
        assert!(!invalid_cookies.is_valid());
    }

    #[test]
    fn test_youtube_cookies_to_cookie_header() {
        let cookies = create_test_cookies();
        let header = cookies.to_cookie_header();

        assert!(header.contains("SID=test_sid"));
        assert!(header.contains("HSID=test_hsid"));
        assert!(header.contains("SSID=test_ssid"));
        assert!(header.contains("APISID=test_apisid"));
        assert!(header.contains("SAPISID=test_sapisid"));
    }

    #[test]
    fn test_cookie_manager_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CookieManager::new(temp_dir.path().to_path_buf());

        let cookies = create_test_cookies();

        // 保存
        manager.save(&cookies).unwrap();

        // 読み込み
        let loaded = manager.load().unwrap();

        assert_eq!(loaded.sid, cookies.sid);
        assert_eq!(loaded.hsid, cookies.hsid);
        assert_eq!(loaded.ssid, cookies.ssid);
        assert_eq!(loaded.apisid, cookies.apisid);
        assert_eq!(loaded.sapisid, cookies.sapisid);
    }

    #[test]
    fn test_cookie_manager_exists() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CookieManager::new(temp_dir.path().to_path_buf());

        assert!(!manager.exists());

        let cookies = create_test_cookies();
        manager.save(&cookies).unwrap();

        assert!(manager.exists());
    }

    #[test]
    fn test_cookie_manager_delete() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CookieManager::new(temp_dir.path().to_path_buf());

        let cookies = create_test_cookies();
        manager.save(&cookies).unwrap();
        assert!(manager.exists());

        manager.delete().unwrap();
        assert!(!manager.exists());
    }

    #[test]
    fn test_cookie_manager_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CookieManager::new(temp_dir.path().to_path_buf());

        let result = manager.load();
        assert!(result.is_err());
    }

    #[test]
    fn test_cookie_manager_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("dir");
        let manager = CookieManager::new(nested_path);

        let cookies = create_test_cookies();
        manager.save(&cookies).unwrap();

        assert!(manager.exists());
    }

    #[test]
    fn test_cookie_config_toml_format() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CookieManager::new(temp_dir.path().to_path_buf());

        let cookies = create_test_cookies();
        manager.save(&cookies).unwrap();

        // TOMLファイルの内容を確認
        let content = fs::read_to_string(manager.config_path()).unwrap();
        assert!(content.contains("[youtube]"));
        assert!(content.contains("sid = \"test_sid\""));
        assert!(content.contains("sapisid = \"test_sapisid\""));
    }
}
