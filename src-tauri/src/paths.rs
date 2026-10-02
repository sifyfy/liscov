//! アプリケーションパス解決モジュール
//!
//! アプリ名・設定ディレクトリ・データディレクトリなどのパスを一元管理する。
//! テスト時は環境変数でオーバーライド可能。

use std::path::PathBuf;

/// 環境変数が無いときのアプリ名・キーリングサービス名。
/// ユニットテストでは本番と別の名前にし、環境変数の設定漏れで本番データに触れないようにする
/// （CLAUDE.md セキュリティ要件）。統合テスト・E2E は cfg(test) にならないので各自で環境変数を設定する。
#[cfg(not(test))]
const DEFAULT_NAME: &str = "liscov-tauri";
#[cfg(test)]
const DEFAULT_NAME: &str = "liscov-test-unit";

/// アプリ名を返す（環境変数 LISCOV_APP_NAME でオーバーライド可能）
pub fn app_name() -> String {
    std::env::var("LISCOV_APP_NAME").unwrap_or_else(|_| DEFAULT_NAME.to_string())
}

/// キーリングサービス名を返す（環境変数 LISCOV_KEYRING_SERVICE でオーバーライド可能）
pub fn keyring_service() -> String {
    std::env::var("LISCOV_KEYRING_SERVICE").unwrap_or_else(|_| DEFAULT_NAME.to_string())
}

/// 設定ディレクトリのパスを返す（OS標準の config_dir + app_name）
pub fn config_dir() -> Result<PathBuf, String> {
    let base =
        dirs::config_dir().ok_or_else(|| "設定ディレクトリを特定できませんでした".to_string())?;
    Ok(base.join(app_name()))
}

/// データディレクトリのパスを返す（OS標準の data_dir + app_name）
pub fn data_dir() -> Result<PathBuf, String> {
    let base =
        dirs::data_dir().ok_or_else(|| "データディレクトリを特定できませんでした".to_string())?;
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

/// ログディレクトリのパスを返す（data_dir + "logs"）
pub fn log_dir() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("logs"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // -----------------------------------------------------------------------
    // app_name
    // -----------------------------------------------------------------------

    #[test]
    #[serial(liscov_env)]
    fn app_name_in_unit_tests_is_not_production() {
        // 環境変数を設定し忘れたユニットテストが本番の認証情報・DB・設定に触れないこと
        // （CLAUDE.md セキュリティ要件。auth のテストが本番の credentials.toml を消した実例あり）
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        assert_ne!(app_name(), "liscov-tauri");
    }

    #[test]
    #[serial(liscov_env)]
    fn app_name_respects_env_override() {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
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
    #[serial(liscov_env)]
    fn keyring_service_in_unit_tests_is_not_production() {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::remove_var("LISCOV_KEYRING_SERVICE") };
        assert_ne!(keyring_service(), "liscov-tauri");
    }

    #[test]
    #[serial(liscov_env)]
    fn keyring_service_respects_env_override() {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
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
    #[serial(liscov_env)]
    fn config_dir_ends_with_app_name() {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = config_dir().expect("config_dir should succeed");
        assert!(
            path.ends_with(app_name()),
            "config_dir should end with app_name, got: {:?}",
            path
        );
    }

    #[test]
    #[serial(liscov_env)]
    fn data_dir_ends_with_app_name() {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = data_dir().expect("data_dir should succeed");
        assert!(
            path.ends_with(app_name()),
            "data_dir should end with app_name, got: {:?}",
            path
        );
    }

    #[test]
    #[serial(liscov_env)]
    fn credentials_path_ends_with_credentials_toml() {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = credentials_path().expect("credentials_path should succeed");
        assert!(path.ends_with("credentials.toml"));
    }

    #[test]
    #[serial(liscov_env)]
    fn config_path_ends_with_config_toml() {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = config_path().expect("config_path should succeed");
        assert!(path.ends_with("config.toml"));
    }

    #[test]
    #[serial(liscov_env)]
    fn database_path_ends_with_liscov_db() {
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        let path = database_path().expect("database_path should succeed");
        assert!(path.ends_with("liscov.db"));
    }

    #[test]
    #[serial(liscov_env)]
    fn log_dir_is_logs_under_data_dir() {
        // FEATURE_SPECIFICATION.md: ログは LISCOV_APP_NAME に従うデータディレクトリの logs/
        // SAFETY: テスト環境でのみ実行。#[serial] で直列化済み
        unsafe { std::env::set_var("LISCOV_APP_NAME", "liscov-test") };
        let path = log_dir();
        let data = data_dir();
        unsafe { std::env::remove_var("LISCOV_APP_NAME") };
        assert_eq!(path.unwrap(), data.unwrap().join("logs"));
    }
}
