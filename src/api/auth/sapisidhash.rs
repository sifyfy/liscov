//! SAPISIDHASH生成モジュール
//!
//! InnerTube APIの認証に必要なSAPISSIDHASHを生成します。
//!
//! ## フォーマット
//!
//! ```text
//! SAPISIDHASH = {timestamp}_{SHA1_HASH}
//! SHA1_HASH = SHA1("{timestamp} {SAPISID} {origin}")
//! ```
//!
//! ## 使用例
//!
//! ```
//! use liscov::api::auth::generate_sapisidhash;
//!
//! let sapisid = "your_sapisid_value";
//! let hash = generate_sapisidhash(sapisid);
//! // => "1234567890_a1b2c3d4e5f6..."
//! ```

use sha1::{Digest, Sha1};
use std::time::{SystemTime, UNIX_EPOCH};

/// YouTube InnerTube APIのデフォルトorigin
const YOUTUBE_ORIGIN: &str = "https://www.youtube.com";

/// SAPISSIDHASHを生成する
///
/// # Arguments
///
/// * `sapisid` - SAPISID Cookieの値
///
/// # Returns
///
/// `{timestamp}_{sha1_hash}` 形式の文字列
pub fn generate_sapisidhash(sapisid: &str) -> String {
    let timestamp = get_current_timestamp();
    generate_sapisidhash_with_timestamp(sapisid, timestamp)
}

/// 指定したタイムスタンプでSAPISSIDHASHを生成する（テスト用）
///
/// # Arguments
///
/// * `sapisid` - SAPISID Cookieの値
/// * `timestamp` - UNIXタイムスタンプ（秒）
///
/// # Returns
///
/// `{timestamp}_{sha1_hash}` 形式の文字列
pub fn generate_sapisidhash_with_timestamp(sapisid: &str, timestamp: u64) -> String {
    generate_sapisidhash_with_origin(sapisid, timestamp, YOUTUBE_ORIGIN)
}

/// カスタムoriginでSAPISSIDHASHを生成する
///
/// # Arguments
///
/// * `sapisid` - SAPISID Cookieの値
/// * `timestamp` - UNIXタイムスタンプ（秒）
/// * `origin` - リクエストのorigin（例: "https://www.youtube.com"）
///
/// # Returns
///
/// `{timestamp}_{sha1_hash}` 形式の文字列
pub fn generate_sapisidhash_with_origin(sapisid: &str, timestamp: u64, origin: &str) -> String {
    let input = format!("{} {} {}", timestamp, sapisid, origin);

    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);

    format!("{}_{}", timestamp, hash_hex)
}

/// 現在のUNIXタイムスタンプを取得
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sapisidhash_format() {
        // 固定値でテスト
        let sapisid = "ABCDEFGHIJKLMNOP";
        let timestamp = 1704067200u64; // 2024-01-01 00:00:00 UTC

        let result = generate_sapisidhash_with_timestamp(sapisid, timestamp);

        // フォーマットを確認: {timestamp}_{40文字のhex}
        let parts: Vec<&str> = result.split('_').collect();
        assert_eq!(parts.len(), 2, "Should have timestamp_hash format");
        assert_eq!(parts[0], "1704067200", "Timestamp should match");
        assert_eq!(parts[1].len(), 40, "SHA1 hash should be 40 hex chars");

        // hex文字のみであることを確認
        assert!(
            parts[1].chars().all(|c| c.is_ascii_hexdigit()),
            "Hash should only contain hex digits"
        );
    }

    #[test]
    fn test_generate_sapisidhash_deterministic() {
        // 同じ入力には同じ出力
        let sapisid = "TEST_SAPISID_VALUE";
        let timestamp = 1704067200u64;

        let result1 = generate_sapisidhash_with_timestamp(sapisid, timestamp);
        let result2 = generate_sapisidhash_with_timestamp(sapisid, timestamp);

        assert_eq!(result1, result2, "Same input should produce same output");
    }

    #[test]
    fn test_generate_sapisidhash_different_inputs() {
        let timestamp = 1704067200u64;

        let result1 = generate_sapisidhash_with_timestamp("SAPISID_A", timestamp);
        let result2 = generate_sapisidhash_with_timestamp("SAPISID_B", timestamp);

        assert_ne!(result1, result2, "Different SAPISID should produce different hash");
    }

    #[test]
    fn test_generate_sapisidhash_different_timestamps() {
        let sapisid = "TEST_SAPISID";

        let result1 = generate_sapisidhash_with_timestamp(sapisid, 1704067200);
        let result2 = generate_sapisidhash_with_timestamp(sapisid, 1704067201);

        assert_ne!(result1, result2, "Different timestamp should produce different hash");
    }

    #[test]
    fn test_generate_sapisidhash_known_value() {
        // 既知の入力と出力でテスト
        // Input: "1704067200 TESTSAPISID https://www.youtube.com"
        // SHA1 hash of this string
        let sapisid = "TESTSAPISID";
        let timestamp = 1704067200u64;

        let result = generate_sapisidhash_with_timestamp(sapisid, timestamp);

        // SHA1("1704067200 TESTSAPISID https://www.youtube.com")を計算
        let expected_input = "1704067200 TESTSAPISID https://www.youtube.com";
        let mut hasher = Sha1::new();
        hasher.update(expected_input.as_bytes());
        let expected_hash = hex::encode(hasher.finalize());

        assert_eq!(
            result,
            format!("1704067200_{}", expected_hash),
            "Should match expected hash"
        );
    }

    #[test]
    fn test_generate_sapisidhash_with_custom_origin() {
        let sapisid = "TEST";
        let timestamp = 1704067200u64;
        let origin = "https://music.youtube.com";

        let result = generate_sapisidhash_with_origin(sapisid, timestamp, origin);

        // 異なるoriginでは異なるハッシュになることを確認
        let youtube_result = generate_sapisidhash_with_timestamp(sapisid, timestamp);
        assert_ne!(result, youtube_result, "Different origin should produce different hash");
    }

    #[test]
    fn test_generate_sapisidhash_empty_sapisid() {
        // 空のSAPISIDでもパニックしない
        let result = generate_sapisidhash_with_timestamp("", 1704067200);
        assert!(!result.is_empty(), "Should produce a result even with empty SAPISID");
    }

    #[test]
    fn test_generate_sapisidhash_realtime() {
        // 現在時刻でのテスト
        let sapisid = "REAL_TEST";
        let result = generate_sapisidhash(sapisid);

        // タイムスタンプが現在時刻付近であることを確認
        let parts: Vec<&str> = result.split('_').collect();
        let timestamp: u64 = parts[0].parse().expect("Should parse timestamp");
        let now = get_current_timestamp();

        // 1秒以内の差であることを確認
        assert!(
            timestamp <= now && timestamp >= now - 1,
            "Timestamp should be within 1 second of now"
        );
    }
}
