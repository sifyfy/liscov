//! ライブリアクション（emojiFountainDataEntity）のパース
//!
//! See: docs/specs/02_chat.md「ライブリアクション」

use crate::core::models::ReactionUpdate;
use serde_json::Value;
use std::collections::BTreeMap;

/// InnerTube API レスポンスからライブリアクションの更新を取り出す（0 件の更新は含めない）
pub fn parse_reaction_updates(data: &Value) -> Vec<ReactionUpdate> {
    data.pointer("/frameworkUpdates/entityBatchUpdate/mutations")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|m| m.pointer("/payload/emojiFountainDataEntity"))
        .filter_map(parse_entity)
        .collect()
}

/// 1 つの emojiFountainDataEntity のバケットを合算する
///
/// バケットの時間順は応答から判別できないため、1 秒ごとの時刻は付けず更新単位にまとめる。
/// 件数は reactionsData だけから数え、totalReactions・intensityScore は使わない。
fn parse_entity(entity: &Value) -> Option<ReactionUpdate> {
    let Some(update_time_usec) = entity
        .get("updateTimeUsec")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
    else {
        tracing::warn!("updateTimeUsec の無いリアクション更新を捨てる（形式変更の可能性）");
        return None;
    };

    let buckets = entity
        .get("reactionBuckets")
        .and_then(|v| v.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default();

    // 観測ではすべて 1 秒。duration が無いバケットも 1 秒として数える
    let duration_seconds = buckets
        .iter()
        .map(|b| {
            b.pointer("/duration/seconds")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1)
        })
        .sum();

    let counts = buckets
        .iter()
        .filter_map(|b| b.get("reactionsData")?.as_array())
        .flatten()
        .filter_map(|r| {
            let emoji = r.get("unicodeEmojiId")?.as_str()?;
            let count = u32::try_from(r.get("reactionCount")?.as_u64()?).ok()?;
            Some((emoji, count))
        })
        .fold(BTreeMap::new(), |mut acc, (emoji, count)| {
            *acc.entry(emoji.to_string()).or_insert(0) += count;
            acc
        });

    ReactionUpdate::new(update_time_usec, duration_seconds, counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn response(entities: Vec<Value>) -> Value {
        let mutations: Vec<Value> = entities
            .into_iter()
            .map(|e| json!({"entityKey": "k", "type": "ENTITY_MUTATION_TYPE_REPLACE", "payload": {"emojiFountainDataEntity": e}}))
            .collect();
        json!({"frameworkUpdates": {"entityBatchUpdate": {"mutations": mutations}}})
    }

    fn bucket(reactions: &[(&str, u32)]) -> Value {
        let data: Vec<Value> = reactions
            .iter()
            .map(|(e, c)| json!({"unicodeEmojiId": e, "reactionCount": c}))
            .collect();
        let total: u32 = reactions.iter().map(|(_, c)| c).sum();
        json!({"duration": {"seconds": "1"}, "intensityScore": 0.75, "reactionsData": data, "totalReactions": total})
    }

    fn empty_bucket() -> Value {
        json!({"duration": {"seconds": "1"}, "intensityScore": 1, "totalReactions": 0})
    }

    fn counts(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
        pairs.iter().map(|(e, c)| (e.to_string(), *c)).collect()
    }

    fn parse_one(entity: Value) -> Option<ReactionUpdate> {
        let updates = parse_reaction_updates(&response(vec![entity]));
        assert!(updates.len() <= 1);
        updates.into_iter().next()
    }

    // 02_chat.md パース例 1: バケット 2 つを合算する
    #[test]
    fn sums_buckets() {
        let update = parse_one(json!({
            "updateTimeUsec": "1790422357025983",
            "reactionBuckets": [bucket(&[("🎉", 4)]), bucket(&[("🎉", 4)])]
        }))
        .unwrap();
        assert_eq!(update.update_time_usec(), 1790422357025983);
        assert_eq!(update.duration_seconds(), 2);
        assert_eq!(update.counts(), &counts(&[("🎉", 8)]));
        assert_eq!(update.total(), 8);
    }

    // 02_chat.md パース例 2: 0 件のバケットも秒数には数える
    #[test]
    fn counts_empty_bucket_duration() {
        let update = parse_one(json!({
            "updateTimeUsec": "1",
            "reactionBuckets": [bucket(&[("❤", 3), ("😄", 1)]), empty_bucket()]
        }))
        .unwrap();
        assert_eq!(update.duration_seconds(), 2);
        assert_eq!(update.counts(), &counts(&[("❤", 3), ("😄", 1)]));
        assert_eq!(update.total(), 4);
    }

    // 02_chat.md: すべてのバケットが 0 件なら捨てる
    #[test]
    fn drops_update_without_reactions() {
        let entity = json!({"updateTimeUsec": "1", "reactionBuckets": [empty_bucket()]});
        assert_eq!(parse_one(entity), None);
    }

    // 02_chat.md: unicodeEmojiId か reactionCount が無い要素だけ無視する
    #[test]
    fn ignores_incomplete_reaction_entries() {
        let update = parse_one(json!({
            "updateTimeUsec": "1",
            "reactionBuckets": [{
                "duration": {"seconds": "1"},
                "reactionsData": [
                    {"reactionCount": 2},
                    {"unicodeEmojiId": "💯"},
                    {"unicodeEmojiId": "😳", "reactionCount": 1}
                ]
            }]
        }))
        .unwrap();
        assert_eq!(update.counts(), &counts(&[("😳", 1)]));
    }

    // 02_chat.md: duration が無いバケットは 1 秒として数える
    #[test]
    fn bucket_without_duration_counts_as_one_second() {
        let update = parse_one(json!({
            "updateTimeUsec": "1",
            "reactionBuckets": [
                {"reactionsData": [{"unicodeEmojiId": "❤", "reactionCount": 1}]},
                bucket(&[("❤", 1)])
            ]
        }))
        .unwrap();
        assert_eq!(update.duration_seconds(), 2);
    }

    // 02_chat.md: updateTimeUsec が無ければ捨てる
    #[test]
    fn drops_update_without_time() {
        let entity = json!({"reactionBuckets": [bucket(&[("❤", 1)])]});
        assert_eq!(parse_one(entity), None);
    }

    // 02_chat.md: total は reactionsData の合計（totalReactions は使わない）
    #[test]
    fn total_ignores_total_reactions_field() {
        let update = parse_one(json!({
            "updateTimeUsec": "1",
            "reactionBuckets": [{
                "duration": {"seconds": "1"},
                "reactionsData": [{"unicodeEmojiId": "❤", "reactionCount": 2}],
                "totalReactions": 99
            }]
        }))
        .unwrap();
        assert_eq!(update.total(), 2);
    }

    // リアクション以外の mutation やチャットだけの応答からは何も出ない
    #[test]
    fn ignores_other_payloads() {
        let data = json!({
            "continuationContents": {"liveChatContinuation": {"actions": []}},
            "frameworkUpdates": {"entityBatchUpdate": {"mutations": [
                {"entityKey": "x", "payload": {"engagementToolbarStateEntityPayload": {}}}
            ]}}
        });
        assert!(parse_reaction_updates(&data).is_empty());
        assert!(parse_reaction_updates(&json!({})).is_empty());
    }
}
