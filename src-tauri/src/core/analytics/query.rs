//! DB から数える・読む（07_revenue.md「集計の対象」）

use super::*;
use crate::core::GiftDetails;
use crate::errors::CommandError;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// 分析・エクスポートの対象のセッション群を、SQL の `json_each(?)` に渡す JSON 配列にする
pub(crate) fn session_ids_json(session_ids: &[String]) -> String {
    serde_json::to_string(session_ids).expect("文字列の配列は必ず JSON にできる")
}

/// この起動で接続したセッション（07_revenue.md「集計の対象」の「現在」）
pub(crate) fn sessions_started_since(
    conn: &rusqlite::Connection,
    since: DateTime<Utc>,
) -> Result<Vec<String>, CommandError> {
    let db_err = |e: rusqlite::Error| CommandError::DatabaseError(e.to_string());
    let mut stmt = conn
        .prepare(
            "SELECT id FROM sessions WHERE julianday(start_time) >= julianday(?1)
             ORDER BY start_time",
        )
        .map_err(db_err)?;
    stmt.query_map([since.to_rfc3339()], |row| row.get(0))
        .map_err(db_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_err)
}

/// 上位貢献者を数えるための DB の集計行（貢献者・種類・色ごと）
pub(crate) struct ContributorRow {
    pub channel_id: String,
    // その組み合わせで最初に保存した行の表示名と id
    pub display_name: String,
    pub first_row_id: i64,
    pub message_type: String,
    pub superchat_color: Option<String>,
    pub count: usize,
}

/// 集計行から上位 10 人を決める（07_revenue.md「上位貢献者」）
///
/// 件数は SuperChat と SuperSticker の合計。件数の多い順、同数なら最高 tier の高い順。
/// 表示名はその視聴者の最初の SuperChat・SuperSticker のときの名前。
pub(crate) fn top_contributors(rows: Vec<ContributorRow>) -> Vec<ContributorInfo> {
    let mut by_channel: HashMap<String, (i64, ContributorInfo)> = HashMap::new();
    for row in rows {
        let tier = (row.message_type == "superchat")
            .then(|| tier_from_header_color(row.superchat_color.as_deref()))
            .flatten();
        let (first_row_id, info) = by_channel.entry(row.channel_id.clone()).or_insert_with(|| {
            (
                row.first_row_id,
                ContributorInfo {
                    channel_id: row.channel_id,
                    display_name: row.display_name.clone(),
                    super_chat_count: 0,
                    highest_tier: None,
                },
            )
        });
        if row.first_row_id < *first_row_id {
            *first_row_id = row.first_row_id;
            info.display_name = row.display_name;
        }
        info.super_chat_count += row.count;
        // 段階不明（None）は比べない
        info.highest_tier = info.highest_tier.max(tier);
    }

    let mut contributors: Vec<ContributorInfo> =
        by_channel.into_values().map(|(_, info)| info).collect();
    contributors.sort_by(|a, b| {
        b.super_chat_count
            .cmp(&a.super_chat_count)
            .then_with(|| b.highest_tier.cmp(&a.highest_tier))
            .then_with(|| a.channel_id.cmp(&b.channel_id))
    });
    contributors.truncate(10);
    contributors
}

/// SuperChat・SuperSticker を貢献者・種類・色ごとに数える
pub(crate) fn query_contributor_rows(
    conn: &rusqlite::Connection,
    session_ids_json: &str,
) -> Result<Vec<ContributorRow>, CommandError> {
    let db_err = |e: rusqlite::Error| CommandError::DatabaseError(e.to_string());
    // MIN(id) と並べた author は、その最初の行の値になる（SQLite の集約の仕様）
    let mut stmt = conn
        .prepare(
            "SELECT channel_id, author, MIN(id), message_type, superchat_color, COUNT(*)
             FROM messages
             WHERE session_id IN (SELECT value FROM json_each(?1))
               AND message_type IN ('superchat', 'supersticker')
             GROUP BY channel_id, message_type, superchat_color",
        )
        .map_err(db_err)?;
    stmt.query_map([session_ids_json], |row| {
        Ok(ContributorRow {
            channel_id: row.get(0)?,
            display_name: row.get(1)?,
            first_row_id: row.get(2)?,
            message_type: row.get(3)?,
            superchat_color: row.get(4)?,
            count: row.get(5)?,
        })
    })
    .map_err(db_err)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(db_err)
}

/// DB の集計行から RevenueAnalytics を計算する純粋関数
///
/// 各行は (message_type, superchat_color, 件数)。superchat_color は色を保存する前の行では NULL（段階不明）
pub(crate) fn compute_session_analytics_from_rows(
    rows: &[(String, Option<String>, usize)],
) -> RevenueAnalytics {
    let mut analytics = RevenueAnalytics::default();

    for (message_type, superchat_color, count) in rows {
        match message_type.as_str() {
            "superchat" => {
                analytics.super_chat_count += count;
                let tier = tier_from_header_color(superchat_color.as_deref());
                analytics.super_chat_by_tier.add(tier, *count);
            }
            "supersticker" => analytics.super_sticker_count += count,
            "membership" | "membership_gift" => analytics.membership_gains += count,
            _ => {}
        }
    }

    analytics
}

/// セッション群の分析を DB から集計する（07_revenue.md「集計の対象」）
pub(crate) fn sessions_analytics(
    conn: &rusqlite::Connection,
    session_ids: &[String],
) -> Result<RevenueAnalytics, CommandError> {
    let db_err = |e: rusqlite::Error| CommandError::DatabaseError(e.to_string());
    let ids = session_ids_json(session_ids);
    // 件数だけ要るので、種類と色ごとに DB で数える（メッセージを全件読まない）
    let mut stmt = conn
        .prepare(
            "SELECT message_type, superchat_color, COUNT(*) FROM messages
             WHERE session_id IN (SELECT value FROM json_each(?1))
             GROUP BY message_type, superchat_color",
        )
        .map_err(db_err)?;
    let rows: Vec<(String, Option<String>, usize)> = stmt
        .query_map([&ids], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(db_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_err)?;

    let mut analytics = compute_session_analytics_from_rows(&rows);
    analytics.top_contributors = top_contributors(query_contributor_rows(conn, &ids)?);
    analytics.gifts = gift_stats_from_metadata(&query_gift_metadata(conn, &ids)?);
    Ok(analytics)
}

/// セッション群の gift 行の metadata を取得する
pub(crate) fn query_gift_metadata(
    conn: &rusqlite::Connection,
    session_ids_json: &str,
) -> Result<Vec<Option<String>>, CommandError> {
    let mut stmt = conn
        .prepare(
            "SELECT metadata FROM messages
             WHERE session_id IN (SELECT value FROM json_each(?1)) AND message_type = 'gift'",
        )
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?;
    stmt.query_map([session_ids_json], |row| row.get::<_, Option<String>>(0))
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CommandError::DatabaseError(e.to_string()))
}

/// 過去セッションのエクスポート用データを DB から読む（07_revenue.md `export_session_data`）
pub(crate) fn session_export_data(
    conn: &rusqlite::Connection,
    session_id: &str,
    config: &ExportConfig,
) -> Result<SessionExportData, CommandError> {
    let session = conn
        .query_row(
            "SELECT id, start_time, end_time, stream_url, stream_title,
                    broadcaster_channel_id, broadcaster_name
             FROM sessions WHERE id = ?",
            [session_id],
            |row| {
                Ok(SessionMetadata {
                    session_id: row.get(0)?,
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                    stream_url: row.get(3)?,
                    stream_title: row.get(4)?,
                    broadcaster_channel_id: row.get(5)?,
                    broadcaster_name: row.get(6)?,
                    export_time: Utc::now().to_rfc3339(),
                })
            },
        )
        .map_err(|e| CommandError::NotFound(format!("Session not found: {}", e)))?;

    let (messages, statistics) = export_messages(conn, &[session_id.to_string()], config)?;

    Ok(SessionExportData {
        metadata: session,
        messages,
        statistics,
    })
}

/// セッション群のメッセージを時系列順に読み、統計と合わせて返す（07_revenue.md エクスポート）
///
/// ギフトの統計は `max_records` で切る前の全件から数える。
pub(crate) fn export_messages(
    conn: &rusqlite::Connection,
    session_ids: &[String],
    config: &ExportConfig,
) -> Result<(Vec<ExportMessage>, SessionStatistics), CommandError> {
    let db_err = |e: rusqlite::Error| CommandError::DatabaseError(e.to_string());
    let ids = session_ids_json(session_ids);

    // LIMIT -1 は SQLite で「上限なし」
    let limit = config.max_records.map_or(-1, |n| n as i64);
    let mut stmt = conn
        .prepare(
            "SELECT message_id, timestamp, author, channel_id, content, message_type, amount,
                    is_member, is_moderator, is_verified, badges, superchat_color, metadata
             FROM messages WHERE session_id IN (SELECT value FROM json_each(?1))
             ORDER BY timestamp, id LIMIT ?2",
        )
        .map_err(db_err)?;

    let messages: Vec<ExportMessage> = stmt
        .query_map(rusqlite::params![ids, limit], |row| {
            let message_type: String = row.get(5)?;
            let amount: Option<String> = row.get(6)?;
            let superchat_color: Option<String> = row.get(11)?;
            let badges_json: Option<String> = row.get(10)?;
            let metadata_json: Option<String> = row.get(12)?;

            let tier = (message_type == "superchat")
                .then(|| tier_from_header_color(superchat_color.as_deref()))
                .flatten();
            let amount_display = if message_type == "gift" {
                metadata_json
                    .and_then(|j| serde_json::from_str::<GiftDetails>(&j).ok())
                    .and_then(|g| g.jewel_count)
                    .map(format_jewels)
            } else {
                amount
            };

            Ok(ExportMessage {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                author: row.get(2)?,
                author_id: row.get(3)?,
                content: row.get(4)?,
                amount_display,
                message_type,
                tier,
                is_member: row.get(7)?,
                // NULL は不明（バッジを保存する前の行・バッジを読めない種類。08_database.md）
                is_moderator: row.get(8)?,
                is_verified: row.get(9)?,
                badges: badges_json.and_then(|j| serde_json::from_str(&j).ok()),
            })
        })
        .map_err(db_err)?
        .collect::<Result<_, _>>()
        .map_err(db_err)?;

    let gifts = gift_stats_from_metadata(&query_gift_metadata(conn, &ids)?);
    let statistics = calculate_session_statistics(&messages, gifts);
    Ok((messages, statistics))
}
