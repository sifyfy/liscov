//! Continuation Token Builder and Modifier
//!
//! YouTubeライブチャットのcontinuation tokenの中のチャットモードを読み書きする。
//! See: docs/specs/02_chat.md「InnerTubeClient内部の切り替え方式」

use base64::{Engine as _, engine::general_purpose};
use std::ops::Range;

use crate::core::models::ChatMode;

/// token 内でチャットモードに至る protobuf の経路: field 119693434 → field 16 → field 1
const OUTER_FIELD: u64 = 119_693_434;
const CHAT_MODE_PARENT_FIELD: u64 = 16;
const CHAT_MODE_FIELD: u64 = 1;

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

/// token を読む。`=` は `%3D` で届くことがあり、パディングの有無はどちらもありうる
fn decode_token(token: &str) -> Option<Vec<u8>> {
    let unpadded = token.replace("%3D", "=");
    general_purpose::URL_SAFE_NO_PAD
        .decode(unpadded.trim_end_matches('='))
        .ok()
}

/// varint を読み、(値, 次の位置) を返す
fn read_varint(bytes: &[u8], mut pos: usize) -> Option<(u64, usize)> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *bytes.get(pos)?;
        pos += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((value, pos));
        }
    }
    None
}

/// `range` 内のメッセージから `field` の値の範囲を探す（最初に現れたもの）
///
/// 値の範囲は、length-delimited なら中身、varint ならそのバイト列。
/// 途中で protobuf として読めなくなったら None。
fn find_field(bytes: &[u8], range: Range<usize>, field: u64) -> Option<Range<usize>> {
    let mut pos = range.start;
    while pos < range.end {
        let (key, after_key) = read_varint(bytes, pos)?;
        let value = match key & 0x7 {
            0 => after_key..read_varint(bytes, after_key)?.1,
            1 => after_key..after_key + 8,
            2 => {
                let (len, start) = read_varint(bytes, after_key)?;
                start..start.checked_add(usize::try_from(len).ok()?)?
            }
            5 => after_key..after_key + 4,
            _ => return None,
        };
        if value.end > range.end {
            return None;
        }
        if key >> 3 == field {
            return Some(value);
        }
        pos = value.end;
    }
    None
}

/// チャットモードの値（1 バイトの varint）の位置を protobuf の構造でたどって探す
///
/// バイト列の見た目で探すと、時刻など別の値の中にある同じバイト列を書き換えてしまうため、構造でたどる。
fn find_chat_mode_offset(bytes: &[u8]) -> Option<usize> {
    let outer = find_field(bytes, 0..bytes.len(), OUTER_FIELD)?;
    let parent = find_field(bytes, outer, CHAT_MODE_PARENT_FIELD)?;
    let value = find_field(bytes, parent, CHAT_MODE_FIELD)?;
    (value.len() == 1).then_some(value.start)
}

/// 既存のcontinuation tokenのチャットモードを書き換える
///
/// 変えるのはチャットモードの 1 バイトだけ。URL-safe base64（パディング無し）で書き戻す。
/// token が想定の形式でなければ None。
pub fn modify_continuation_mode(original: &str, new_mode: ChatMode) -> Option<String> {
    let mut bytes = decode_token(original)?;
    let Some(offset) = find_chat_mode_offset(&bytes) else {
        tracing::warn!("continuation token にチャットモードが見つからない（形式変更の可能性）");
        return None;
    };
    bytes[offset] = chat_mode_to_type(new_mode);
    Some(general_purpose::URL_SAFE_NO_PAD.encode(&bytes))
}

/// 既存のcontinuation tokenから現在のチャットモードを検出
pub fn detect_chat_mode(token: &str) -> Option<ChatMode> {
    let bytes = decode_token(token)?;
    chat_type_to_mode(bytes[find_chat_mode_offset(&bytes)?])
}

/// テスト用: 実データと同じ構造の continuation token を作る
///
/// field 119693434 の中に、field 16 より前に置く囮（field 3 の bytes に 82 01 02 08 04 を含む）と
/// 時刻の varint、field 16 {1: chattype, 3: 1} を持つ。
/// `padded` なら 32 バイト（base64 で `=` が 1 つ付く → 実データと同じく `%3D`）、
/// そうでなければ 36 バイト（パディング無し）にする。
#[cfg(test)]
pub(crate) fn test_token(chattype: u8, padded: bool) -> String {
    let decoy = [0x82, 0x01, 0x02, 0x08, 0x04];
    let mut inner = vec![0x1a, decoy.len() as u8];
    inner.extend_from_slice(&decoy);
    // field 5: 時刻（varint）
    inner.extend_from_slice(&[0x28, 0xd4, 0xc6, 0x8e, 0xa7, 0xd7, 0xa3, 0xe5, 0x03]);
    if !padded {
        // field 6: 0、field 8: 1（長さを 3 の倍数にしてパディングを無くす）
        inner.extend_from_slice(&[0x30, 0x00, 0x40, 0x01]);
    }
    // field 16: {1: chattype, 3: 1}
    inner.extend_from_slice(&[0x82, 0x01, 0x04, 0x08, chattype, 0x18, 0x01]);
    // field 17: 0
    inner.extend_from_slice(&[0x88, 0x01, 0x00]);
    let mut bytes = vec![0xd2, 0x87, 0xcc, 0xc8, 0x03, inner.len() as u8];
    bytes.extend_from_slice(&inner);
    general_purpose::URL_SAFE.encode(&bytes).replace('=', "%3D")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(token: &str) -> Vec<u8> {
        general_purpose::URL_SAFE_NO_PAD
            .decode(token.replace("%3D", "").trim_end_matches('='))
            .unwrap()
    }

    // 02_chat.md: field 16 の field 1 = 4 の token を AllChat に → field 1 だけが 1 になる
    // （field 16 より前にある囮の 82 01 02 08 04 は書き換えない）
    #[test]
    fn modifies_only_chat_mode_byte() {
        let modified = modify_continuation_mode(&test_token(4, false), ChatMode::AllChat).unwrap();
        assert_eq!(decode(&modified), decode(&test_token(1, false)));
    }

    // 02_chat.md: 末尾が %3D の token も書き換えられる
    #[test]
    fn modifies_percent_encoded_padded_token() {
        let original = test_token(4, true);
        assert!(original.ends_with("%3D"));

        let modified = modify_continuation_mode(&original, ChatMode::AllChat).unwrap();

        assert_eq!(detect_chat_mode(&original), Some(ChatMode::TopChat));
        assert_eq!(detect_chat_mode(&modified), Some(ChatMode::AllChat));
    }

    #[test]
    fn modifies_all_chat_back_to_top_chat() {
        let modified = modify_continuation_mode(&test_token(1, true), ChatMode::TopChat).unwrap();
        assert_eq!(detect_chat_mode(&modified), Some(ChatMode::TopChat));
    }

    // 02_chat.md: すでに目的のモード → そのまま成功
    #[test]
    fn already_target_mode_succeeds() {
        let modified = modify_continuation_mode(&test_token(1, false), ChatMode::AllChat).unwrap();
        assert_eq!(detect_chat_mode(&modified), Some(ChatMode::AllChat));
    }

    // 02_chat.md: 経路が無い・protobuf として読めない → 失敗
    #[test]
    fn fails_without_path() {
        // field 119693434 はあるが field 16 が無い
        let bytes = [0xd2, 0x87, 0xcc, 0xc8, 0x03, 0x02, 0x30, 0x00];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        assert_eq!(modify_continuation_mode(&token, ChatMode::AllChat), None);
        assert_eq!(detect_chat_mode(&token), None);
    }

    #[test]
    fn fails_on_broken_token() {
        // 長さが実データより長い（途中で切れている）
        let bytes = [
            0xd2, 0x87, 0xcc, 0xc8, 0x03, 0x40, 0x82, 0x01, 0x02, 0x08, 0x04,
        ];
        let token = general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        assert_eq!(modify_continuation_mode(&token, ChatMode::AllChat), None);
        assert_eq!(
            modify_continuation_mode("not base64 !!", ChatMode::AllChat),
            None
        );
    }

    #[test]
    fn detects_unknown_value_as_none() {
        assert_eq!(detect_chat_mode(&test_token(2, false)), None);
    }
}
