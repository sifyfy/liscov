//! Continuation Token Builder and Modifier
//!
//! YouTubeライブチャットのcontinuation tokenを変更・構築するモジュール。
//! 既存トークンのchattypeフィールドを変更することでモード切替を実現。

use base64::{engine::general_purpose, Engine as _};

use crate::core::models::ChatMode;

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
/// * `Some(String)` - 変更成功時、新しいトークン
/// * `None` - 変更失敗時（トークン形式が予期しない場合）
pub fn modify_continuation_mode(original: &str, new_mode: ChatMode) -> Option<String> {
    let new_chattype = chat_mode_to_type(new_mode);

    // Base64デコード（URL安全形式と標準形式の両方に対応）
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(original)
        .or_else(|_| general_purpose::STANDARD.decode(original))
        .ok()?;

    tracing::debug!(
        "Token to modify: length={}, first 20 bytes={:02x?}",
        decoded.len(),
        &decoded[..20.min(decoded.len())]
    );

    let mut modified = decoded.clone();
    let mut found = false;

    // chattype値を探す（0x01 または 0x04）
    // 様々なパターンを試す:
    // 1. Field 16: 0x82 0x01 + length + 0x08 + chattype
    // 2. Field 13: 0x68 + chattype
    // 3. バイト列 0x08 の後に 0x01 または 0x04

    // パターン1: Field 16 内の nested field 1
    for i in 0..modified.len().saturating_sub(4) {
        if modified[i] == 0x82 && modified[i + 1] == 0x01 {
            let len = modified[i + 2] as usize;
            if i + 3 + len <= modified.len() && modified[i + 3] == 0x08 {
                let old_val = modified[i + 4];
                if old_val == 0x01 || old_val == 0x04 {
                    tracing::debug!(
                        "Modifying chattype at offset {} (pattern 1): {} -> {}",
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

    // パターン2: Field 16 with varint length > 127 (0x82 0x01 + 2-byte length)
    // Longer tokens may use 2-byte varint for length
    if !found {
        for i in 0..modified.len().saturating_sub(5) {
            if modified[i] == 0x82 && modified[i + 1] == 0x01 {
                // Check if next byte could be start of a 2-byte varint (high bit set)
                if modified[i + 2] & 0x80 != 0 {
                    // 2-byte varint length: skip 2 bytes for length
                    if i + 5 < modified.len() && modified[i + 4] == 0x08 {
                        let old_val = modified[i + 5];
                        if old_val == 0x01 || old_val == 0x04 {
                            tracing::debug!(
                                "Modifying chattype at offset {} (pattern 2 - 2byte len): {} -> {}",
                                i + 5,
                                old_val,
                                new_chattype
                            );
                            modified[i + 5] = new_chattype;
                            found = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    // パターン3: 0x08 + chattype の後に 0x10 (field 2) が続くパターン
    // これはchattypeフィールドの典型的なコンテキスト
    if !found {
        for i in 0..modified.len().saturating_sub(3) {
            if modified[i] == 0x08 {
                let val = modified[i + 1];
                if (val == 0x01 || val == 0x04) && modified[i + 2] == 0x10 {
                    tracing::debug!(
                        "Modifying chattype at offset {} (pattern 3 - 08 chattype 10): {} -> {}",
                        i + 1,
                        val,
                        new_chattype
                    );
                    modified[i + 1] = new_chattype;
                    found = true;
                    break;
                }
            }
        }
    }

    // パターン4: 最後の手段 - length-delimited field内で 0x08 01/04 を探す
    if !found {
        for i in 0..modified.len().saturating_sub(2) {
            if modified[i] == 0x08 {
                let val = modified[i + 1];
                if val == 0x01 || val == 0x04 {
                    // 前のバイトがlength-delimitedフィールドの長さとして妥当か確認
                    if i >= 3 {
                        let prev = modified[i - 1];
                        // 長さが2（0x08 + value）で、このフィールドが長さ2の内容の開始位置にある
                        if prev == 0x02 || prev == 0x03 || prev == 0x04 {
                            tracing::debug!(
                                "Modifying chattype at offset {} (pattern 4 - fallback): {} -> {}",
                                i + 1,
                                val,
                                new_chattype
                            );
                            modified[i + 1] = new_chattype;
                            found = true;
                            break;
                        }
                    }
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
        Some(encoded)
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
pub fn detect_chat_mode(token: &str) -> Option<ChatMode> {
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .or_else(|_| general_purpose::STANDARD.decode(token))
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

        // TopChat -> AllChat
        let modified = modify_continuation_mode(&original, ChatMode::AllChat);
        assert!(modified.is_some());

        let modified_token = modified.unwrap();
        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(&modified_token).unwrap();
        // chattype is at offset 11 (i=7, i+4=11)
        assert_eq!(decoded[11], 0x01); // chattype should be 1 now

        // AllChat -> TopChat
        let reverted = modify_continuation_mode(&modified_token, ChatMode::TopChat);
        assert!(reverted.is_some());

        let reverted_token = reverted.unwrap();
        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(&reverted_token).unwrap();
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
        let top_token = general_purpose::URL_SAFE_NO_PAD.encode(&inner_top);
        assert_eq!(detect_chat_mode(&top_token), Some(ChatMode::TopChat));

        // AllChat token
        let inner_all = vec![
            0xd2, 0x87, 0xcc, 0xc8, 0x03,
            0x10, 0x00,
            0x82, 0x01, 0x02, 0x08, 0x01, // chattype=1 (AllChat)
            0x20, 0x00,
        ];
        let all_token = general_purpose::URL_SAFE_NO_PAD.encode(&inner_all);
        assert_eq!(detect_chat_mode(&all_token), Some(ChatMode::AllChat));
    }

    #[test]
    fn test_modify_invalid_token_returns_none() {
        // chattype field がないトークン
        let inner = vec![0xd2, 0x87, 0xcc, 0xc8, 0x03, 0x10, 0x00];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::AllChat);
        assert!(result.is_none());
    }

    // spec: Pattern2 (2-byte varint length) でchattypeを変更できる
    #[test]
    fn test_modify_pattern2_topchat_to_allchat() {
        // 0x82 0x01 + 2-byte varint (0x82 0x02) + 0x08 + chattype=4(TopChat)
        let inner = vec![
            0x82, 0x01, // Field 16 marker
            0x82, 0x02, // 2-byte varint length (high bit set)
            0x08, 0x04, // chattype = 4 (TopChat)
            0x00,       // padding
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::AllChat);
        assert!(result.is_some());

        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(result.unwrap()).unwrap();
        // i=0, chattype at i+5=5
        assert_eq!(decoded[5], 0x01);
    }

    // spec: Pattern2 で AllChat -> TopChat に変更できる
    #[test]
    fn test_modify_pattern2_allchat_to_topchat() {
        let inner = vec![
            0x82, 0x01,
            0x82, 0x02,
            0x08, 0x01, // chattype = 1 (AllChat)
            0x00,
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::TopChat);
        assert!(result.is_some());

        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(result.unwrap()).unwrap();
        assert_eq!(decoded[5], 0x04);
    }

    // spec: Pattern3 (0x08 chattype 0x10) でchattypeを変更できる
    #[test]
    fn test_modify_pattern3_topchat_to_allchat() {
        // Pattern1/2 マーカー (0x82 0x01) を含まないトークン
        // 0x08 + chattype=4 + 0x10 のパターン
        let inner = vec![
            0xAA, 0xBB, // ダミーバイト (0x82 0x01 でない)
            0x08, 0x04, // chattype = 4 (TopChat)
            0x10, 0x20, // 0x10 マーカー
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::AllChat);
        assert!(result.is_some());

        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(result.unwrap()).unwrap();
        // i=2, chattype at i+1=3
        assert_eq!(decoded[3], 0x01);
    }

    // spec: Pattern3 で AllChat -> TopChat に変更できる
    #[test]
    fn test_modify_pattern3_allchat_to_topchat() {
        let inner = vec![
            0xCC, 0xDD,
            0x08, 0x01, // chattype = 1 (AllChat)
            0x10, 0x00,
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::TopChat);
        assert!(result.is_some());

        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(result.unwrap()).unwrap();
        assert_eq!(decoded[3], 0x04);
    }

    // spec: Pattern4 (fallback, 前バイトが 0x02/0x03/0x04) でchattypeを変更できる
    #[test]
    fn test_modify_pattern4_allchat_prev_byte_0x02() {
        // Pattern1/2/3 に一致しない構造、前バイト=0x02, i >= 3 を満たす
        let inner = vec![
            0xFF, 0xEE, 0xDD, // パディング (i >= 3 を満たす)
            0x02,             // prev byte = 0x02 (length)
            0x08, 0x01,       // chattype = 1 (AllChat)
            0x00,             // ダミー
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::TopChat);
        assert!(result.is_some());

        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(result.unwrap()).unwrap();
        // i=4, chattype at i+1=5
        assert_eq!(decoded[5], 0x04);
    }

    // spec: Pattern4 で前バイトが 0x03 の場合もchattypeを変更できる
    #[test]
    fn test_modify_pattern4_topchat_prev_byte_0x03() {
        let inner = vec![
            0xFF, 0xEE, 0xDD,
            0x03,
            0x08, 0x04, // chattype = 4 (TopChat)
            0x00,
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::AllChat);
        assert!(result.is_some());

        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(result.unwrap()).unwrap();
        assert_eq!(decoded[5], 0x01);
    }

    // spec: Pattern4 で前バイトが 0x04 の場合もchattypeを変更できる
    #[test]
    fn test_modify_pattern4_prev_byte_0x04() {
        let inner = vec![
            0xFF, 0xEE, 0xDD,
            0x04,
            0x08, 0x01,
            0x00,
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::TopChat);
        assert!(result.is_some());

        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(result.unwrap()).unwrap();
        assert_eq!(decoded[5], 0x04);
    }

    // spec: Pattern4 で前バイトが条件(0x02/0x03/0x04)を満たさない場合は None を返す
    #[test]
    fn test_modify_pattern4_invalid_prev_byte_returns_none() {
        // Pattern1/2/3 に一致せず、前バイトが 0x01 (条件外)
        let inner = vec![
            0xFF, 0xEE, 0xDD,
            0x01, // prev byte = 0x01 (条件外)
            0x08, 0x04,
            0x00,
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::AllChat);
        assert!(result.is_none());
    }

    // spec: Field 13 (0x68) も一緒に更新される
    #[test]
    fn test_modify_field13_updated_together_with_field16() {
        // Pattern1 (Field 16) と Field 13 の両方を含むトークン
        let inner = vec![
            0x82, 0x01, 0x02, 0x08, 0x04, // Field 16: chattype=4 (TopChat)
            0x68, 0x04,                    // Field 13: chattype=4 (TopChat)
            0x10, 0x00,                    // ダミー
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::AllChat);
        assert!(result.is_some());

        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(result.unwrap()).unwrap();
        // Field 16 の chattype (offset 4) が変更されている
        assert_eq!(decoded[4], 0x01);
        // Field 13 の chattype (offset 6) も変更されている
        assert_eq!(decoded[6], 0x01);
    }

    // spec: Field 13 が AllChat の場合も TopChat に更新される
    #[test]
    fn test_modify_field13_allchat_to_topchat() {
        let inner = vec![
            0x82, 0x01, 0x02, 0x08, 0x01, // Field 16: chattype=1 (AllChat)
            0x68, 0x01,                    // Field 13: chattype=1 (AllChat)
            0x10, 0x00,
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        let result = modify_continuation_mode(&token, ChatMode::TopChat);
        assert!(result.is_some());

        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(result.unwrap()).unwrap();
        assert_eq!(decoded[4], 0x04);
        assert_eq!(decoded[6], 0x04);
    }

    // spec: 0x82 は含むが直後が 0x01 でないトークンは detect_chat_mode で None を返す
    #[test]
    fn test_detect_chat_mode_0x82_without_0x01_returns_none() {
        let inner = vec![
            0x82, 0x02, // 0x82 の後が 0x01 でない
            0x02, 0x08, 0x04,
            0x00,
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        assert_eq!(detect_chat_mode(&token), None);
    }

    // spec: Pattern3の `&&` mutant対策 - 次バイトが 0x10 でない場合はマッチしない
    // `(val == 0x01 || val == 0x04) && modified[i + 2] == 0x10` の `&&` が `||` に
    // なると、次バイト無関係でマッチしてしまうため None が返ることを検証する
    #[test]
    fn test_modify_pattern3_does_not_match_wrong_context_byte() {
        // Pattern3: next byte = 0x20 (0x10 でない) → マッチしない
        let inner = vec![0xAA, 0xBB, 0x08, 0x04, 0x20, 0x00];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);
        let result = modify_continuation_mode(&token, ChatMode::AllChat);
        assert!(result.is_none());
    }

    // spec: chattype が 1/4 以外のトークンは detect_chat_mode で None を返す
    #[test]
    fn test_detect_chat_mode_unknown_chattype_returns_none() {
        let inner = vec![
            0x82, 0x01, 0x02, 0x08, 0x02, // chattype=2 (未定義)
            0x00,
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(&inner);

        assert_eq!(detect_chat_mode(&token), None);
    }
}
