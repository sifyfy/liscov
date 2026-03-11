//! アプリケーションパス解決モジュール
//!
//! アプリ名・設定ディレクトリ・データディレクトリなどのパスを一元管理する。
//! テスト時は環境変数でオーバーライド可能。

use std::path::PathBuf;

/// アプリ名を返す（環境変数 LISCOV_APP_NAME でオーバーライド可能）
pub fn app_name() -> String {
    std::env::var("LISCOV_APP_NAME").unwrap_or_else(|_| "liscov-tauri".to_string())
}

/// キーリングサービス名を返す（環境変数 LISCOV_KEYRING_SERVICE でオーバーライド可能）
pub fn keyring_service() -> String {
    std::env::var("LISCOV_KEYRING_SERVICE").unwrap_or_else(|_| "liscov-tauri".to_string())
}

/// 設定ディレクトリのパスを返す（OS標準の config_dir + app_name）
pub fn config_dir() -> Result<PathBuf, String> {
    let base = dirs::config_dir()
        .ok_or_else(|| "設定ディレクトリを特定できませんでした".to_string())?;
    Ok(base.join(app_name()))
}

/// データディレクトリのパスを返す（OS標準の data_dir + app_name）
pub fn data_dir() -> Result<PathBuf, String> {
    let base = dirs::data_dir()
        .ok_or_else(|| "データディレクトリを特定できませんでした".to_string())?;
    Ok(base.join(app_name()))
}

/// 認証情報ファイルのパスを返す（config_dir + "credentials.toml"）
pub fn credentials_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("credentials.toml"))
}

/// 設定ファイルのパスを返す（config_dir + "config.toml"）
pub fn config_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("config.toml"))
}

/// データベースファイルのパスを返す（data_dir + "liscov.db"）
pub fn database_path() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("liscov.db"))
}

/// バックアップディレクトリのパスを返す（data_dir + "backups"）
pub fn backup_dir() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("backups"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // app_name
    // -----------------------------------------------------------------------

    #[test]
    fn app_name_returns_default_when_env_not_set() {
        // 環境変数が未設定のとき既定値 "liscov-tauri" を返す
        // SAFETY: テスト環境でのみ実行。並列テストとの競合を避けるため単一スレッドで確認
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        assert_eq!(app_name(), "liscov-tauri");
    }

    #[test]
    fn app_name_respects_env_override() {
        // テスト用に環境変数をセットしてオーバーライドを確認
        // SAFETY: テスト環境でのみ実行
        unsafe {
            std::env::set_var("LISCOV_APP_NAME", "liscov-test");
        }
        let result = app_name();
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        assert_eq!(result, "liscov-test");
    }

    // -----------------------------------------------------------------------
    // keyring_service
    // -----------------------------------------------------------------------

    #[test]
    fn keyring_service_returns_default_when_env_not_set() {
        // SAFETY: テスト環境でのみ実行
        unsafe { std::env::remove_var("LISCOV_KEYRING_SERVICE") };
        assert_eq!(keyring_service(), "liscov-tauri");
    }

    #[test]
    fn keyring_service_respects_env_override() {
        // SAFETY: テスト環境でのみ実行
        unsafe {
            std::env::set_var("LISCOV_KEYRING_SERVICE", "liscov-test");
        }
        let result = keyring_service();
        unsafe { std::env::remove_var("LISCOV_KEYRING_SERVICE") };
        assert_eq!(result, "liscov-test");
    }

    // -----------------------------------------------------------------------
    // パスの構成確認（末尾がapp_nameで終わる）
    // -----------------------------------------------------------------------

    #[test]
    fn config_dir_ends_with_app_name() {
        // SAFETY: テスト環境でのみ実行
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = config_dir().expect("config_dir should succeed");
        assert!(
            path.ends_with("liscov-tauri"),
            "config_dir should end with app_name, got: {:?}",
            path
        );
    }

    #[test]
    fn data_dir_ends_with_app_name() {
        // SAFETY: テスト環境でのみ実行
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = data_dir().expect("data_dir should succeed");
        assert!(
            path.ends_with("liscov-tauri"),
            "data_dir should end with app_name, got: {:?}",
            path
        );
    }

    #[test]
    fn credentials_path_ends_with_credentials_toml() {
        // SAFETY: テスト環境でのみ実行
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = credentials_path().expect("credentials_path should succeed");
        assert!(path.ends_with("credentials.toml"));
    }

    #[test]
    fn config_path_ends_with_config_toml() {
        // SAFETY: テスト環境でのみ実行
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = config_path().expect("config_path should succeed");
        assert!(path.ends_with("config.toml"));
    }

    #[test]
    fn database_path_ends_with_liscov_db() {
        // SAFETY: テスト環境でのみ実行
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = database_path().expect("database_path should succeed");
        assert!(path.ends_with("liscov.db"));
    }

    #[test]
    fn backup_dir_ends_with_backups() {
        // SAFETY: テスト環境でのみ実行
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = backup_dir().expect("backup_dir should succeed");
        assert!(path.ends_with("backups"));
    }
}
