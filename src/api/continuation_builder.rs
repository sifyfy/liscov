//! Continuation Token Builder and Modifier
//!
//! YouTubeライブチャットのcontinuation tokenを変更・構築するモジュール。
//! 既存トークンのchattypeフィールドを変更することでモード切替を実現。

use base64::{engine::general_purpose, Engine as _};

use crate::api::youtube::{ChatMode, Continuation};

/// チャットモードをchattype値に変換
fn chat_mode_to_type(mode: ChatMode) -> u8 {
    match mode {
        ChatMode::TopChat => 4,
        ChatMode::AllChat => 1,
    }
}

/// chattype値をチャットモードに変換
fn chat_type_to_mode(chattype: u8) -> Option<ChatMode> {
    match chattype {
        4 => Some(ChatMode::TopChat),
        1 => Some(ChatMode::AllChat),
        _ => None,
    }
}

/// 既存のcontinuation tokenを変更して新しいモードのトークンを生成
///
/// YouTubeのcontinuation tokenはProtocol Buffer形式でエンコードされている。
/// このフィールド構造:
/// - Field 16 (0x82 0x01): length-delimited, ネストされたメッセージ
///   - Field 1 (0x08): chattype値 (1=AllChat, 4=TopChat)
///
/// # Arguments
/// * `original` - 元のcontinuation token (Base64エンコード済み)
/// * `new_mode` - 変更後のチャットモード
///
/// # Returns
/// * `Some(Continuation)` - 変更成功時、新しいトークン
/// * `None` - 変更失敗時（トークン形式が予期しない場合）
pub fn modify_continuation_mode(original: &Continuation, new_mode: ChatMode) -> Option<Continuation> {
    let new_chattype = chat_mode_to_type(new_mode);

    // Base64デコード（URL安全形式と標準形式の両方に対応）
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(&original.0)
        .or_else(|_| general_purpose::STANDARD.decode(&original.0))
        .ok()?;

    let mut modified = decoded.clone();
    let mut found = false;

    // Field 16 内の nested field 1 を探す
    // パターン: 0x82 0x01 (field 16) + length + 0x08 (field 1) + chattype
    for i in 0..modified.len().saturating_sub(4) {
        if modified[i] == 0x82 && modified[i + 1] == 0x01 {
            let len = modified[i + 2] as usize;
            if i + 3 + len <= modified.len() && modified[i + 3] == 0x08 {
                let old_val = modified[i + 4];
                if old_val == 0x01 || old_val == 0x04 {
                    tracing::debug!(
                        "Modifying chattype at offset {}: {} -> {}",
                        i + 4,
                        old_val,
                        new_chattype
                    );
                    modified[i + 4] = new_chattype;
                    found = true;
                    break;
                }
            }
        }
    }

    // Field 13も変更（存在する場合）
    for i in 0..modified.len().saturating_sub(1) {
        if modified[i] == 0x68 {
            let old_val = modified[i + 1];
            if old_val == 0x01 || old_val == 0x04 {
                tracing::debug!(
                    "Modifying field 13 at offset {}: {} -> {}",
                    i + 1,
                    old_val,
                    new_chattype
                );
                modified[i + 1] = new_chattype;
            }
        }
    }

    if found {
        // Base64エンコード（URL安全形式）
        let encoded = general_purpose::URL_SAFE_NO_PAD.encode(&modified);
        Some(Continuation(encoded))
    } else {
        tracing::warn!("Could not find chattype field in continuation token");
        None
    }
}

/// 既存のcontinuation tokenから現在のチャットモードを検出
///
/// # Arguments
/// * `token` - continuation token (Base64エンコード済み)
///
/// # Returns
/// * `Some(ChatMode)` - 検出成功時
/// * `None` - 検出失敗時
pub fn detect_chat_mode(token: &Continuation) -> Option<ChatMode> {
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(&token.0)
        .or_else(|_| general_purpose::STANDARD.decode(&token.0))
        .ok()?;

    // Field 16 内の nested field 1 を探す
    for i in 0..decoded.len().saturating_sub(4) {
        if decoded[i] == 0x82 && decoded[i + 1] == 0x01 {
            let len = decoded[i + 2] as usize;
            if i + 3 + len <= decoded.len() && decoded[i + 3] == 0x08 {
                let chattype = decoded[i + 4];
                return chat_type_to_mode(chattype);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_mode_to_type() {
        assert_eq!(chat_mode_to_type(ChatMode::TopChat), 4);
        assert_eq!(chat_mode_to_type(ChatMode::AllChat), 1);
    }

    #[test]
    fn test_chat_type_to_mode() {
        assert_eq!(chat_type_to_mode(4), Some(ChatMode::TopChat));
        assert_eq!(chat_type_to_mode(1), Some(ChatMode::AllChat));
        assert_eq!(chat_type_to_mode(0), None);
        assert_eq!(chat_type_to_mode(2), None);
    }

    #[test]
    fn test_modify_token_roundtrip() {
        // 実際のトークン構造をシミュレート
        // Field 16 (0x82 0x01) + length(2) + Field 1 (0x08) + value(4=TopChat)
        let inner = vec![
            0xd2, 0x87, 0xcc, 0xc8, 0x03, // YouTube header
            0x10, 0x00, // some field
            0x82, 0x01, 0x02, 0x08, 0x04, // Field 16 with chattype=4
            0x20, 0x00, // trailing field
        ];
        let original = general_purpose::URL_SAFE_NO_PAD.encode(&inner);
        let original_token = Continuation(original);

        // TopChat -> AllChat
        let modified = modify_continuation_mode(&original_token, ChatMode::AllChat);
        assert!(modified.is_some());

        let modified_token = modified.unwrap();
        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(&modified_token.0).unwrap();
        // chattype is at offset 11 (i=7, i+4=11)
        assert_eq!(decoded[11], 0x01); // chattype should be 1 now

        // AllChat -> TopChat
        let reverted = modify_continuation_mode(&modified_token, ChatMode::TopChat);
        assert!(reverted.is_some());

        let reverted_token = reverted.unwrap();
        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(&reverted_token.0).unwrap();
        assert_eq!(decoded[11], 0x04); // chattype should be 4 again
    }

    #[test]
    fn test_detect_chat_mode() {
        // TopChat token
        let inner_top = vec![
            0xd2, 0x87, 0xcc, 0xc8, 0x03,
            0x10, 0x00,
            0x82, 0x01, 0x02, 0x08, 0x04, // chattype=4 (TopChat)
            0x20, 0x00,
        ];
        let top_token = Continuation(general_purpose::URL_SAFE_NO_PAD.encode(&inner_top));
        assert_eq!(detect_chat_mode(&top_token), Some(ChatMode::TopChat));

        // AllChat token
        let inner_all = vec![
            0xd2, 0x87, 0xcc, 0xc8, 0x03,
            0x10, 0x00,
            0x82, 0x01, 0x02, 0x08, 0x01, // chattype=1 (AllChat)
            0x20, 0x00,
        ];
        let all_token = Continuation(general_purpose::URL_SAFE_NO_PAD.encode(&inner_all));
        assert_eq!(detect_chat_mode(&all_token), Some(ChatMode::AllChat));
    }

    #[test]
    fn test_modify_invalid_token_returns_none() {
        // chattype field がないトークン
        let inner = vec![0xd2, 0x87, 0xcc, 0xc8, 0x03, 0x10, 0x00];
        let token = Continuation(general_purpose::URL_SAFE_NO_PAD.encode(&inner));

        let result = modify_continuation_mode(&token, ChatMode::AllChat);
        assert!(result.is_none());
    }
}
