//! アプリケーション共通エラー型
//!
//! Tauri コマンドのエラーをフロントエンドに構造化して伝達する。
//! JSON シリアライズ時は { "kind": "ErrorVariant", "message": "詳細" } の形式。

use serde::Serialize;

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", content = "message")]
pub enum CommandError {
    /// 認証が必要（未ログイン）
    #[error("{0}")]
    AuthRequired(String),
    /// 認証失敗（無効なクレデンシャル）
    #[error("{0}")]
    AuthFailed(String),
    /// 認証情報ストレージ操作失敗
    #[error("{0}")]
    StorageError(String),
    /// ネットワーク接続失敗
    #[error("{0}")]
    ConnectionFailed(String),
    /// 未接続状態での操作
    #[error("{0}")]
    NotConnected(String),
    /// データベース操作エラー
    #[error("{0}")]
    DatabaseError(String),
    /// リソースが見つからない
    #[error("{0}")]
    NotFound(String),
    /// HTTP/API呼び出しエラー
    #[error("{0}")]
    ApiError(String),
    /// TTS操作エラー
    #[error("{0}")]
    TtsError(String),
    /// 入力値が不正
    #[error("{0}")]
    InvalidInput(String),
    /// ファイルI/Oエラー
    #[error("{0}")]
    IoError(String),
    /// その他の内部エラー
    #[error("{0}")]
    Internal(String),
}

impl From<anyhow::Error> for CommandError {
    fn from(e: anyhow::Error) -> Self {
        CommandError::Internal(e.to_string())
    }
}

impl From<crate::tts::TtsError> for CommandError {
    fn from(e: crate::tts::TtsError) -> Self {
        CommandError::TtsError(e.to_string())
    }
}

impl From<rusqlite::Error> for CommandError {
    fn from(e: rusqlite::Error) -> Self {
        CommandError::DatabaseError(e.to_string())
    }
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        CommandError::IoError(e.to_string())
    }
}

impl From<reqwest::Error> for CommandError {
    fn from(e: reqwest::Error) -> Self {
        CommandError::ApiError(e.to_string())
    }
}
