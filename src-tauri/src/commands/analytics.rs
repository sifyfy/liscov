//! Analytics and export commands
//!
//! Implements 07_revenue.md specification
//! Note: SuperChat amounts are NOT calculated numerically due to different currencies.
//! Instead, we use tier-based aggregation based on YouTube's color scheme.

use crate::core::{ChatMessage, GiftDetails, MessageType};
use crate::errors::CommandError;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tauri::State;
use ts_rs::TS;

/// SuperChat tier based on YouTube color scheme
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub enum SuperChatTier {
    Blue,    // Lowest tier (USD $1-2)
    Cyan,    // USD $2-5
    Green,   // USD $5-10
    Yellow,  // USD $10-20
    Orange,  // USD $20-50
    Magenta, // USD $50-100
    Red,     // Highest tier (USD $100-500)
}

/// SuperChat tier statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct SuperChatTierStats {
    pub tier_red: usize,
    pub tier_magenta: usize,
    pub tier_orange: usize,
    pub tier_yellow: usize,
    pub tier_green: usize,
    pub tier_cyan: usize,
    pub tier_blue: usize,
    // 段階不明（ヘッダー色が表に無い・色が分からない）
    pub tier_unknown: usize,
}

impl SuperChatTierStats {
    /// 1 件のスーパーチャットを数える。段階が分からなければ None
    pub fn record(&mut self, tier: Option<SuperChatTier>) {
        self.add(tier, 1);
    }

    pub fn increment(&mut self, tier: SuperChatTier) {
        self.add(Some(tier), 1);
    }

    /// 同じ段階のスーパーチャットを count 件数える。段階が分からなければ None
    pub fn add(&mut self, tier: Option<SuperChatTier>, count: usize) {
        let slot = match tier {
            Some(SuperChatTier::Red) => &mut self.tier_red,
            Some(SuperChatTier::Magenta) => &mut self.tier_magenta,
            Some(SuperChatTier::Orange) => &mut self.tier_orange,
            Some(SuperChatTier::Yellow) => &mut self.tier_yellow,
            Some(SuperChatTier::Green) => &mut self.tier_green,
            Some(SuperChatTier::Cyan) => &mut self.tier_cyan,
            Some(SuperChatTier::Blue) => &mut self.tier_blue,
            None => &mut self.tier_unknown,
        };
        *slot += count;
    }

    pub fn total(&self) -> usize {
        self.tier_red
            + self.tier_magenta
            + self.tier_orange
            + self.tier_yellow
            + self.tier_green
            + self.tier_cyan
            + self.tier_blue
            + self.tier_unknown
    }
}

/// Revenue analytics data (07_revenue.md)
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct RevenueAnalytics {
    pub super_chat_count: usize,
    pub super_chat_by_tier: SuperChatTierStats,
    pub super_sticker_count: usize,
    pub membership_gains: usize,
    pub hourly_stats: Vec<HourlyStats>,
    pub top_contributors: Vec<ContributorInfo>,
    // ギフト（ジュエル）の別枠集計。既存の集計値には含めない
    pub gifts: GiftStats,
}

/// ギフト集計（07_revenue.md GiftStats）
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct GiftStats {
    pub gift_count: usize,
    // 件数降順、同数はギフト名昇順
    pub gifts_by_name: Vec<GiftNameCount>,
    // ジュエル数が分かったギフトの件数
    pub jewel_known_count: usize,
    // 分かったものだけの合計（不明分は推測しない）
    #[ts(type = "number")]
    pub total_jewels: u64,
}

/// ギフト名ごとの件数
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct GiftNameCount {
    pub gift_name: String,
    // 最初に見たギフトの画像 URL
    pub gift_image_url: Option<String>,
    pub count: usize,
}

impl GiftStats {
    pub(crate) fn from_gifts<'a>(gifts: impl IntoIterator<Item = &'a GiftDetails>) -> Self {
        let mut stats = GiftStats::default();
        for gift in gifts {
            stats.gift_count += 1;
            if let Some(jewels) = gift.jewel_count {
                stats.jewel_known_count += 1;
                stats.total_jewels += u64::from(jewels);
            }
            match stats
                .gifts_by_name
                .iter_mut()
                .find(|g| g.gift_name == gift.gift_name)
            {
                Some(entry) => entry.count += 1,
                None => stats.gifts_by_name.push(GiftNameCount {
                    gift_name: gift.gift_name.clone(),
                    gift_image_url: gift.gift_image_url.clone(),
                    count: 1,
                }),
            }
        }
        stats.gifts_by_name.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.gift_name.cmp(&b.gift_name))
        });
        stats
    }
}

/// メッセージ列に含まれるギフトを取り出す
fn gifts_in(messages: &[ChatMessage]) -> impl Iterator<Item = &GiftDetails> {
    messages.iter().filter_map(|m| match &m.message_type {
        MessageType::Gift(gift) => Some(gift),
        _ => None,
    })
}

/// DB の gift 行の metadata JSON（08_database.md）から集計する。読めない行は数えない
pub(crate) fn gift_stats_from_metadata(rows: &[Option<String>]) -> GiftStats {
    let gifts: Vec<GiftDetails> = rows
        .iter()
        .flatten()
        .filter_map(|json| serde_json::from_str(json).ok())
        .collect();
    GiftStats::from_gifts(&gifts)
}

/// エクスポートの amount_display に入れるジュエル数の表記
fn format_jewels(jewel_count: u32) -> String {
    format!("{} Jewels", jewel_count)
}

/// Contributor information (07_revenue.md)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct ContributorInfo {
    pub channel_id: String,
    pub display_name: String,
    pub super_chat_count: usize,
    pub highest_tier: Option<SuperChatTier>,
}

/// Hourly statistics (07_revenue.md)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct HourlyStats {
    pub hour: String,
    pub super_chat_count: usize,
    pub super_sticker_count: usize,
    pub membership_count: usize,
    pub message_count: usize,
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct ExportConfig {
    pub format: String, // "csv", "json"
    pub include_metadata: bool,
    pub include_system_messages: bool,
    pub max_records: Option<usize>,
    pub sort_order: Option<String>,
}

/// Session statistics for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExportData {
    pub metadata: SessionMetadata,
    pub messages: Vec<ExportMessage>,
    pub statistics: SessionStatistics,
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session_id: String,
    pub stream_title: Option<String>,
    pub stream_url: Option<String>,
    pub broadcaster_name: Option<String>,
    pub broadcaster_channel_id: Option<String>,
    pub start_time: String,
    pub end_time: Option<String>,
    pub export_time: String,
}

/// Export message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMessage {
    pub id: String,
    pub timestamp: String,
    pub author: String,
    pub author_id: String,
    pub content: String,
    pub message_type: String,
    pub amount_display: Option<String>,
    /// superchat 以外と段階不明は None
    pub tier: Option<SuperChatTier>,
    /// None = 不明（バッジを読めない種類・保存する前の過去分。08_database.md）
    pub is_moderator: Option<bool>,
    pub is_member: bool,
    /// None = 不明
    pub is_verified: Option<bool>,
    /// None = 不明
    pub badges: Option<Vec<String>>,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatistics {
    pub total_messages: usize,
    pub unique_viewers: usize,
    pub super_chat_count: usize,
    pub super_chat_by_tier: SuperChatTierStats,
    pub membership_count: usize,
    pub gifts: GiftStats,
}

/// スーパーチャットのヘッダー背景色と段階（07_revenue.md「Tier別集計」の表。実データで確認した色）
const TIER_HEADER_COLORS: [(&str, SuperChatTier); 7] = [
    ("#1565C0", SuperChatTier::Blue),
    ("#00B8D4", SuperChatTier::Cyan),
    ("#00BFA5", SuperChatTier::Green),
    ("#FFB300", SuperChatTier::Yellow),
    ("#E65100", SuperChatTier::Orange),
    ("#C2185B", SuperChatTier::Magenta),
    ("#D00000", SuperChatTier::Red),
];

/// ヘッダー背景色（`#RRGGBB`）から段階を判定する。表に無い色・色が無ければ None（段階不明）
///
/// 金額からは推定しない（通貨が混ざるため。07_revenue.md「制約・不変条件」）。
fn tier_from_header_color(color: Option<&str>) -> Option<SuperChatTier> {
    let color = color?;
    TIER_HEADER_COLORS
        .iter()
        .find(|(header, _)| header.eq_ignore_ascii_case(color))
        .map(|(_, tier)| *tier)
}

/// メッセージリストからRevenueAnalyticsを計算する純粋関数
///
/// SuperChat/SuperSticker/Membershipの集計、貢献者トラッキング、上位10人truncateを行う
pub(crate) fn compute_revenue_analytics(messages: &[ChatMessage]) -> RevenueAnalytics {
    let mut analytics = RevenueAnalytics::default();

    // 貢献者トラッキング: channel_id -> (display_name, count, highest_tier)
    let mut contributors: HashMap<String, (String, usize, Option<SuperChatTier>)> = HashMap::new();

    for message in messages {
        match &message.message_type {
            MessageType::SuperChat { .. } => {
                analytics.super_chat_count += 1;

                let tier = tier_from_header_color(message.superchat_header_color());
                analytics.super_chat_by_tier.record(tier);

                // 貢献者情報を更新
                let entry = contributors.entry(message.channel_id.clone()).or_insert((
                    message.author.clone(),
                    0,
                    None,
                ));
                entry.1 += 1;
                // より高いtierがあれば更新（段階不明は比べない）
                if tier > entry.2 {
                    entry.2 = tier;
                }
            }
            MessageType::SuperSticker { amount: _ } => {
                analytics.super_sticker_count += 1;

                // SuperStickerは件数カウントのみ（tier統計には影響しない）
                let entry = contributors.entry(message.channel_id.clone()).or_insert((
                    message.author.clone(),
                    0,
                    None,
                ));
                entry.1 += 1;
            }
            MessageType::Membership { .. } | MessageType::MembershipGift { .. } => {
                analytics.membership_gains += 1;
            }
            _ => {}
        }
    }

    // 貢献者リストを件数降順→tier降順でソートし上位10人に絞る
    let mut contributors_vec: Vec<ContributorInfo> = contributors
        .into_iter()
        .map(
            |(channel_id, (display_name, super_chat_count, highest_tier))| ContributorInfo {
                channel_id,
                display_name,
                super_chat_count,
                highest_tier,
            },
        )
        .collect();

    contributors_vec.sort_by(|a, b| match b.super_chat_count.cmp(&a.super_chat_count) {
        std::cmp::Ordering::Equal => b.highest_tier.cmp(&a.highest_tier),
        other => other,
    });

    contributors_vec.truncate(10);
    analytics.top_contributors = contributors_vec;
    analytics.gifts = GiftStats::from_gifts(gifts_in(messages));

    analytics
}

/// Get revenue analytics for current session
#[tauri::command]
pub async fn get_revenue_analytics(
    state: State<'_, AppState>,
) -> Result<RevenueAnalytics, CommandError> {
    let messages = state.messages.read().await;
    // VecDequeをVecに変換して純粋関数に渡す
    let messages_vec: Vec<ChatMessage> = messages.iter().cloned().collect();
    Ok(compute_revenue_analytics(&messages_vec))
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

/// 過去セッションの分析を DB から集計する（07_revenue.md `get_session_analytics`）
pub(crate) fn session_analytics(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<RevenueAnalytics, CommandError> {
    // 件数だけ要るので、種類と色ごとに DB で数える（メッセージを全件読まない）
    let mut stmt = conn
        .prepare(
            "SELECT message_type, superchat_color, COUNT(*) FROM messages
             WHERE session_id = ? GROUP BY message_type, superchat_color",
        )
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?;
    let rows: Vec<(String, Option<String>, usize)> = stmt
        .query_map([session_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?;

    let mut analytics = compute_session_analytics_from_rows(&rows);
    analytics.gifts = gift_stats_from_metadata(&query_gift_metadata(conn, session_id)?);
    Ok(analytics)
}

/// Get analytics for a specific session from database
#[tauri::command]
pub async fn get_session_analytics(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<RevenueAnalytics, CommandError> {
    let conn = state.db_connection().await?;
    session_analytics(&conn, &session_id)
}

/// セッション内の gift 行の metadata を取得する
fn query_gift_metadata(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<Option<String>>, CommandError> {
    let mut stmt = conn
        .prepare("SELECT metadata FROM messages WHERE session_id = ? AND message_type = 'gift'")
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?;
    stmt.query_map([session_id], |row| row.get::<_, Option<String>>(0))
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
    let db_err = |e: rusqlite::Error| CommandError::DatabaseError(e.to_string());

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

    // LIMIT -1 は SQLite で「上限なし」
    let limit = config.max_records.map_or(-1, |n| n as i64);
    let mut stmt = conn
        .prepare(
            "SELECT message_id, timestamp, author, channel_id, content, message_type, amount,
                    is_member, is_moderator, is_verified, badges, superchat_color, metadata
             FROM messages WHERE session_id = ?1 ORDER BY timestamp LIMIT ?2",
        )
        .map_err(db_err)?;

    let messages: Vec<ExportMessage> = stmt
        .query_map(rusqlite::params![session_id, limit], |row| {
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

    let gifts = gift_stats_from_metadata(&query_gift_metadata(conn, session_id)?);
    let statistics = calculate_session_statistics(&messages, gifts);

    Ok(SessionExportData {
        metadata: session,
        messages,
        statistics,
    })
}

/// Export session data to file
#[tauri::command]
pub async fn export_session_data(
    state: State<'_, AppState>,
    session_id: String,
    file_path: String,
    config: ExportConfig,
) -> Result<(), CommandError> {
    // ファイルへの書き出しの間、DB のロックを握らない
    let export_data = {
        let conn = state.db_connection().await?;
        session_export_data(&conn, &session_id, &config)?
    };
    write_export(&export_data, &config, &file_path)
}

/// 形式に合わせて整形し、ファイルに書き出す
fn write_export(
    data: &SessionExportData,
    config: &ExportConfig,
    file_path: &str,
) -> Result<(), CommandError> {
    let content = match config.format.as_str() {
        "json" => export_to_json(data, config)?,
        "csv" => export_to_csv(data, config)?,
        _ => {
            return Err(CommandError::InvalidInput(format!(
                "Unsupported format: {}",
                config.format
            )));
        }
    };

    let mut file = File::create(file_path)
        .map_err(|e| CommandError::IoError(format!("Failed to create file: {}", e)))?;
    file.write_all(content.as_bytes())
        .map_err(|e| CommandError::IoError(format!("Failed to write file: {}", e)))
}

/// ChatMessageリストからExportMessageリストへの変換
///
/// 各ChatMessageのmessage_type・metadata・色情報からExportMessage形式に変換する
pub(crate) fn convert_messages_to_export(
    messages: &[ChatMessage],
    _session_id: &str,
    _broadcaster_channel_id: &str,
) -> Vec<ExportMessage> {
    messages
        .iter()
        .map(|msg| {
            let (message_type_str, amount_display, tier) = match &msg.message_type {
                MessageType::Text => ("text".to_string(), None, None),
                MessageType::SuperChat { amount } => (
                    "superchat".to_string(),
                    Some(amount.clone()),
                    tier_from_header_color(msg.superchat_header_color()),
                ),
                MessageType::SuperSticker { amount } => {
                    ("supersticker".to_string(), Some(amount.clone()), None)
                }
                MessageType::Membership { .. } => ("membership".to_string(), None, None),
                MessageType::MembershipGift { .. } => ("membership_gift".to_string(), None, None),
                MessageType::Gift(gift) => (
                    "gift".to_string(),
                    gift.jewel_count.map(format_jewels),
                    None,
                ),
                MessageType::System => ("system".to_string(), None, None),
            };

            // バッジを読めない種類は不明（None）。08_database.md の保存と同じ規則
            let badges = msg.author_badge_metadata();

            ExportMessage {
                id: msg.id.clone(),
                timestamp: msg.timestamp.clone(),
                author: msg.author.clone(),
                author_id: msg.channel_id.clone(),
                content: msg.content.clone(),
                message_type: message_type_str,
                amount_display,
                tier,
                is_moderator: badges.map(|m| m.is_moderator),
                is_member: msg.is_member,
                is_verified: badges.map(|m| m.is_verified),
                badges: badges.map(|m| m.badges.clone()),
            }
        })
        .collect()
}

/// Export current session messages
#[tauri::command]
pub async fn export_current_messages(
    state: State<'_, AppState>,
    file_path: String,
    config: ExportConfig,
) -> Result<(), CommandError> {
    // 必要な分を複製したらすぐ手放す（監視ループの追加を待たせない）
    let messages_vec: Vec<ChatMessage> = state
        .messages
        .read()
        .await
        .iter()
        .take(config.max_records.unwrap_or(usize::MAX))
        .cloned()
        .collect();

    // 多接続モデル: 最初の接続からセッションID・配信者IDを取得（エクスポートヘッダ用）
    let (session_id, broadcaster_id) = {
        let connections = state.connections.read().await;
        let session_id = connections
            .values()
            .find_map(|c| c.session_id.clone())
            .unwrap_or_else(|| "current".to_string());
        let broadcaster_id = connections
            .values()
            .map(|c| &c.broadcaster_channel_id)
            .find(|id| !id.is_empty())
            .cloned()
            .unwrap_or_default();
        (session_id, broadcaster_id)
    };

    let export_messages = convert_messages_to_export(&messages_vec, &session_id, &broadcaster_id);

    let statistics = calculate_session_statistics(
        &export_messages,
        GiftStats::from_gifts(gifts_in(&messages_vec)),
    );

    let export_data = SessionExportData {
        metadata: SessionMetadata {
            session_id,
            stream_title: None,
            stream_url: None,
            broadcaster_name: None,
            broadcaster_channel_id: Some(broadcaster_id).filter(|id| !id.is_empty()),
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            export_time: Utc::now().to_rfc3339(),
        },
        statistics,
        messages: export_messages,
    };
    write_export(&export_data, &config, &file_path)
}

// Helper functions

/// Calculate session statistics from export messages (DRY: used by both export functions)
fn calculate_session_statistics(messages: &[ExportMessage], gifts: GiftStats) -> SessionStatistics {
    let mut unique_viewers: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut super_chat_count = 0;
    let mut super_chat_by_tier = SuperChatTierStats::default();
    let mut membership_count = 0;

    for msg in messages {
        unique_viewers.insert(msg.author_id.clone());

        match msg.message_type.as_str() {
            "superchat" => {
                super_chat_count += 1;
                super_chat_by_tier.record(msg.tier);
            }
            "membership" | "membership_gift" => {
                membership_count += 1;
            }
            _ => {}
        }
    }

    SessionStatistics {
        total_messages: messages.len(),
        unique_viewers: unique_viewers.len(),
        super_chat_count,
        super_chat_by_tier,
        membership_count,
        gifts,
    }
}

fn export_to_json(data: &SessionExportData, config: &ExportConfig) -> Result<String, CommandError> {
    if config.include_metadata {
        serde_json::to_string_pretty(data)
            .map_err(|e| CommandError::Internal(format!("JSON serialization error: {}", e)))
    } else {
        serde_json::to_string_pretty(&data.messages)
            .map_err(|e| CommandError::Internal(format!("JSON serialization error: {}", e)))
    }
}

fn export_to_csv(data: &SessionExportData, config: &ExportConfig) -> Result<String, CommandError> {
    let mut csv = String::new();

    // Metadata header (per spec)
    if config.include_metadata {
        csv.push_str("# Metadata\n");
        csv.push_str(&format!("# Session ID,{}\n", data.metadata.session_id));
        if let Some(ref title) = data.metadata.stream_title {
            csv.push_str(&format!("# Stream Title,{}\n", title));
        }
        if let Some(ref name) = data.metadata.broadcaster_name {
            csv.push_str(&format!("# Channel,{}\n", name));
        }
        if let Some(ref url) = data.metadata.stream_url {
            csv.push_str(&format!("# Stream URL,{}\n", url));
        }
        csv.push_str(&format!("# Start Time,{}\n", data.metadata.start_time));
        if let Some(ref end) = data.metadata.end_time {
            csv.push_str(&format!("# End Time,{}\n", end));
        }
        csv.push_str(&format!(
            "# Total Messages,{}\n",
            data.statistics.total_messages
        ));
        csv.push_str(&format!(
            "# Unique Viewers,{}\n",
            data.statistics.unique_viewers
        ));
        csv.push_str(&format!(
            "# SuperChat Count,{}\n",
            data.statistics.super_chat_count
        ));
        csv.push_str(&format!("# Export Time,{}\n", data.metadata.export_time));
        csv.push('\n');
    }

    // Header (per spec)
    csv.push_str("id,timestamp,author,author_id,content,message_type,amount_display,tier,is_moderator,is_member,is_verified,badges\n");

    // Data rows
    for msg in &data.messages {
        let amount_str = msg.amount_display.as_deref().unwrap_or("");
        let tier_str = msg
            .tier
            .map(|t| format!("{:?}", t).to_lowercase())
            .unwrap_or_default();
        let content_escaped = msg.content.replace('"', "\"\"");
        // 不明（None）は空欄にする（07_revenue.md）
        let badges_str = msg.badges.as_ref().map(|b| b.join(";")).unwrap_or_default();
        let flag = |value: Option<bool>| value.map(|v| v.to_string()).unwrap_or_default();

        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},{},{},\"{}\"\n",
            msg.id,
            msg.timestamp,
            msg.author.replace('"', "\"\""),
            msg.author_id,
            content_escaped,
            msg.message_type,
            amount_str,
            tier_str,
            flag(msg.is_moderator),
            msg.is_member,
            flag(msg.is_verified),
            badges_str
        ));
    }

    Ok(csv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{MessageMetadata, SuperChatColors};

    // ========================================================================
    // tier_from_header_color (07_revenue.md「Tier別集計」の表)
    // ========================================================================

    #[test]
    fn tier_from_header_color_matches_spec_table() {
        let cases = [
            ("#1565C0", SuperChatTier::Blue),
            ("#00B8D4", SuperChatTier::Cyan),
            ("#00BFA5", SuperChatTier::Green),
            ("#FFB300", SuperChatTier::Yellow),
            ("#E65100", SuperChatTier::Orange),
            ("#C2185B", SuperChatTier::Magenta),
            ("#D00000", SuperChatTier::Red),
        ];
        for (color, tier) in cases {
            assert_eq!(tier_from_header_color(Some(color)), Some(tier), "{color}");
        }
    }

    #[test]
    fn tier_from_header_color_ignores_case() {
        assert_eq!(
            tier_from_header_color(Some("#00bfa5")),
            Some(SuperChatTier::Green)
        );
    }

    #[test]
    fn tier_from_header_color_not_in_table_is_unknown() {
        // 本文の背景色（緑 #1DE9B6・赤 #E62117）はヘッダー色ではない
        assert_eq!(tier_from_header_color(Some("#1DE9B6")), None);
        assert_eq!(tier_from_header_color(Some("#E62117")), None);
        assert_eq!(tier_from_header_color(Some("")), None);
    }

    #[test]
    fn tier_from_header_color_without_color_is_unknown() {
        assert_eq!(tier_from_header_color(None), None);
    }

    #[test]
    fn tier_stats_record_unknown() {
        let mut stats = SuperChatTierStats::default();
        stats.record(None);
        stats.record(Some(SuperChatTier::Green));
        assert_eq!(stats.tier_unknown, 1);
        assert_eq!(stats.tier_green, 1);
        assert_eq!(stats.total(), 2);
    }

    // ========================================================================
    // SuperChatTierStats (07_revenue.md: Tier統計)
    // ========================================================================

    #[test]
    fn tier_stats_increment_and_total() {
        let mut stats = SuperChatTierStats::default();
        assert_eq!(stats.total(), 0);

        stats.increment(SuperChatTier::Red);
        stats.increment(SuperChatTier::Red);
        stats.increment(SuperChatTier::Blue);
        stats.increment(SuperChatTier::Yellow);

        assert_eq!(stats.tier_red, 2);
        assert_eq!(stats.tier_blue, 1);
        assert_eq!(stats.tier_yellow, 1);
        assert_eq!(stats.total(), 4);
    }

    #[test]
    fn tier_stats_default_all_zero() {
        let stats = SuperChatTierStats::default();
        assert_eq!(stats.tier_red, 0);
        assert_eq!(stats.tier_magenta, 0);
        assert_eq!(stats.tier_orange, 0);
        assert_eq!(stats.tier_yellow, 0);
        assert_eq!(stats.tier_green, 0);
        assert_eq!(stats.tier_cyan, 0);
        assert_eq!(stats.tier_blue, 0);
        assert_eq!(stats.total(), 0);
    }

    // ========================================================================
    // export_to_csv (07_revenue.md: CSVエクスポート)
    // ========================================================================

    fn make_test_export_data() -> SessionExportData {
        SessionExportData {
            metadata: SessionMetadata {
                session_id: "test-session-1".to_string(),
                stream_title: Some("Test Stream".to_string()),
                stream_url: Some("https://youtube.com/watch?v=test".to_string()),
                broadcaster_name: Some("TestChannel".to_string()),
                broadcaster_channel_id: Some("UC_test".to_string()),
                start_time: "2025-01-14T14:00:00Z".to_string(),
                end_time: Some("2025-01-14T16:00:00Z".to_string()),
                export_time: "2025-01-14T17:00:00Z".to_string(),
            },
            messages: vec![
                ExportMessage {
                    id: "msg1".to_string(),
                    timestamp: "14:00:01".to_string(),
                    author: "User1".to_string(),
                    author_id: "UC_user1".to_string(),
                    content: "Hello".to_string(),
                    message_type: "text".to_string(),
                    amount_display: None,
                    tier: None,
                    is_moderator: Some(false),
                    is_member: false,
                    is_verified: Some(false),
                    badges: Some(vec![]),
                },
                ExportMessage {
                    id: "msg2".to_string(),
                    timestamp: "14:00:05".to_string(),
                    author: "User2".to_string(),
                    author_id: "UC_user2".to_string(),
                    content: "Super Chat!".to_string(),
                    message_type: "superchat".to_string(),
                    amount_display: Some("$10.00".to_string()),
                    tier: Some(SuperChatTier::Yellow),
                    is_moderator: Some(false),
                    is_member: true,
                    is_verified: Some(false),
                    badges: Some(vec!["member".to_string()]),
                },
            ],
            statistics: SessionStatistics {
                total_messages: 2,
                unique_viewers: 2,
                super_chat_count: 1,
                super_chat_by_tier: SuperChatTierStats::default(),
                membership_count: 0,
                gifts: GiftStats::default(),
            },
        }
    }

    #[test]
    fn csv_export_with_metadata() {
        let data = make_test_export_data();
        let config = ExportConfig {
            format: "csv".to_string(),
            include_metadata: true,
            include_system_messages: false,
            max_records: None,
            sort_order: None,
        };

        let csv = export_to_csv(&data, &config).unwrap();

        assert!(csv.starts_with("# Metadata\n"));
        assert!(csv.contains("# Session ID,test-session-1"));
        assert!(csv.contains("# Stream Title,Test Stream"));
        assert!(csv.contains("# Channel,TestChannel"));
        assert!(csv.contains("# Total Messages,2"));
        assert!(csv.contains("# Unique Viewers,2"));
        assert!(csv.contains("# SuperChat Count,1"));
        assert!(csv.contains("id,timestamp,author,author_id,content,message_type,amount_display,tier,is_moderator,is_member,is_verified,badges\n"));
        assert!(csv.contains("\"msg1\""));
        assert!(csv.contains("\"msg2\""));
    }

    #[test]
    fn csv_export_without_metadata() {
        let data = make_test_export_data();
        let config = ExportConfig {
            format: "csv".to_string(),
            include_metadata: false,
            include_system_messages: false,
            max_records: None,
            sort_order: None,
        };

        let csv = export_to_csv(&data, &config).unwrap();

        assert!(!csv.contains("# Metadata"));
        assert!(csv.starts_with("id,timestamp,"));
    }

    #[test]
    fn csv_export_header_matches_spec() {
        let data = make_test_export_data();
        let config = ExportConfig {
            format: "csv".to_string(),
            include_metadata: false,
            include_system_messages: false,
            max_records: None,
            sort_order: None,
        };

        let csv = export_to_csv(&data, &config).unwrap();
        let header_line = csv.lines().next().unwrap();
        assert_eq!(
            header_line,
            "id,timestamp,author,author_id,content,message_type,amount_display,tier,is_moderator,is_member,is_verified,badges"
        );
    }

    #[test]
    fn csv_export_superchat_row_has_tier() {
        let data = make_test_export_data();
        let config = ExportConfig {
            format: "csv".to_string(),
            include_metadata: false,
            include_system_messages: false,
            max_records: None,
            sort_order: None,
        };

        let csv = export_to_csv(&data, &config).unwrap();
        let superchat_line = csv.lines().find(|l| l.contains("msg2")).unwrap();
        assert!(superchat_line.contains("yellow"));
        assert!(superchat_line.contains("$10.00"));
    }

    // ========================================================================
    // export_to_json (07_revenue.md: JSONエクスポート)
    // ========================================================================

    #[test]
    fn json_export_with_metadata() {
        let data = make_test_export_data();
        let config = ExportConfig {
            format: "json".to_string(),
            include_metadata: true,
            include_system_messages: false,
            max_records: None,
            sort_order: None,
        };

        let json = export_to_json(&data, &config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.get("metadata").is_some());
        assert!(parsed.get("messages").is_some());
        assert!(parsed.get("statistics").is_some());
        assert_eq!(parsed["metadata"]["session_id"], "test-session-1");
    }

    #[test]
    fn json_export_without_metadata_returns_messages_only() {
        let data = make_test_export_data();
        let config = ExportConfig {
            format: "json".to_string(),
            include_metadata: false,
            include_system_messages: false,
            max_records: None,
            sort_order: None,
        };

        let json = export_to_json(&data, &config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    // ========================================================================
    // RevenueAnalytics default (07_revenue.md)
    // ========================================================================

    #[test]
    fn revenue_analytics_default() {
        let analytics = RevenueAnalytics::default();
        assert_eq!(analytics.super_chat_count, 0);
        assert_eq!(analytics.super_sticker_count, 0);
        assert_eq!(analytics.membership_gains, 0);
        assert!(analytics.hourly_stats.is_empty());
        assert!(analytics.top_contributors.is_empty());
        assert_eq!(analytics.super_chat_by_tier.total(), 0);
    }

    // ========================================================================
    // SuperChatTierStats::increment - 全Tier個別検証 (07_revenue.md: Tier統計)
    // 対象mutant: L49-53 の += が *= / -= に置換されるケースを検出
    // ========================================================================

    #[test]
    fn tier_stats_increment_magenta() {
        // 07_revenue.md: Magenta tierのincrementが正しく加算されること
        let mut stats = SuperChatTierStats::default();
        stats.increment(SuperChatTier::Magenta);
        assert_eq!(stats.tier_magenta, 1);
    }

    #[test]
    fn tier_stats_increment_orange() {
        // 07_revenue.md: Orange tierのincrementが正しく加算されること
        let mut stats = SuperChatTierStats::default();
        stats.increment(SuperChatTier::Orange);
        assert_eq!(stats.tier_orange, 1);
    }

    #[test]
    fn tier_stats_increment_green() {
        // 07_revenue.md: Green tierのincrementが正しく加算されること
        let mut stats = SuperChatTierStats::default();
        stats.increment(SuperChatTier::Green);
        assert_eq!(stats.tier_green, 1);
    }

    #[test]
    fn tier_stats_increment_cyan() {
        // 07_revenue.md: Cyan tierのincrementが正しく加算されること
        let mut stats = SuperChatTierStats::default();
        stats.increment(SuperChatTier::Cyan);
        assert_eq!(stats.tier_cyan, 1);
    }

    // ========================================================================
    // SuperChatTierStats::total - 全加算項独立検証 (07_revenue.md: Tier統計)
    // 対象mutant: L59-60 の + が - / * に置換されるケースを検出
    // ========================================================================

    #[test]
    fn tier_stats_total_magenta_and_orange() {
        // 07_revenue.md: Magenta×1 + Orange×1 の合計が2になること（L59の加算パス検証）
        let mut stats = SuperChatTierStats::default();
        stats.increment(SuperChatTier::Magenta);
        stats.increment(SuperChatTier::Orange);
        assert_eq!(stats.total(), 2);
    }

    #[test]
    fn tier_stats_total_green_and_cyan() {
        // 07_revenue.md: Green×1 + Cyan×1 の合計が2になること（L60の加算パス検証）
        let mut stats = SuperChatTierStats::default();
        stats.increment(SuperChatTier::Green);
        stats.increment(SuperChatTier::Cyan);
        assert_eq!(stats.total(), 2);
    }

    // ========================================================================
    // calculate_session_statistics (07_revenue.md: セッション統計集計)
    // 対象mutant:
    //   L615: delete match arm "superchat" → super_chat_count がインクリメントされない
    //   L616: super_chat_count += 1 → -= 1 / *= 1
    //   L622: membership_count += 1 → -= 1 / *= 1
    // ========================================================================

    fn make_export_message(
        id: &str,
        author_id: &str,
        message_type: &str,
        tier: Option<SuperChatTier>,
    ) -> ExportMessage {
        ExportMessage {
            id: id.to_string(),
            timestamp: "2025-01-14T14:00:00Z".to_string(),
            author: "TestUser".to_string(),
            author_id: author_id.to_string(),
            content: "test content".to_string(),
            message_type: message_type.to_string(),
            amount_display: None,
            tier,
            is_moderator: Some(false),
            is_member: false,
            is_verified: Some(false),
            badges: Some(vec![]),
        }
    }

    #[test]
    fn session_stats_super_chat_count_increments() {
        // 07_revenue.md: "superchat" メッセージが super_chat_count を1増やすこと
        // L615 (match arm削除mutant) と L616 (+= 1 → -= 1 / *= 1 mutant) を殺す
        let messages = vec![
            make_export_message("sc1", "UC_user1", "superchat", Some(SuperChatTier::Yellow)),
            make_export_message("sc2", "UC_user2", "superchat", Some(SuperChatTier::Red)),
            make_export_message("sc3", "UC_user3", "superchat", Some(SuperChatTier::Blue)),
        ];

        let stats = calculate_session_statistics(&messages, GiftStats::default());

        // 3件のsuperchatが正しく集計される
        assert_eq!(stats.super_chat_count, 3);
    }

    #[test]
    fn session_stats_super_chat_count_not_incremented_for_non_superchat() {
        // 07_revenue.md: "chat" / "text" メッセージは super_chat_count に影響しないこと
        let messages = vec![
            make_export_message("msg1", "UC_user1", "text", None),
            make_export_message("msg2", "UC_user2", "text", None),
        ];

        let stats = calculate_session_statistics(&messages, GiftStats::default());

        assert_eq!(stats.super_chat_count, 0);
    }

    #[test]
    fn session_stats_membership_count_increments() {
        // 07_revenue.md: "membership" メッセージが membership_count を1増やすこと
        // L622 (+= 1 → -= 1 / *= 1 mutant) を殺す
        let messages = vec![
            make_export_message("m1", "UC_user1", "membership", None),
            make_export_message("m2", "UC_user2", "membership", None),
        ];

        let stats = calculate_session_statistics(&messages, GiftStats::default());

        // 2件のmembershipが正しく集計される
        assert_eq!(stats.membership_count, 2);
    }

    #[test]
    fn session_stats_membership_gift_count_increments() {
        // 07_revenue.md: "membership_gift" メッセージが membership_count を1増やすこと
        // L622 の "membership" | "membership_gift" パターン検証
        let messages = vec![
            make_export_message("mg1", "UC_user1", "membership_gift", None),
            make_export_message("mg2", "UC_user2", "membership_gift", None),
            make_export_message("mg3", "UC_user3", "membership_gift", None),
        ];

        let stats = calculate_session_statistics(&messages, GiftStats::default());

        assert_eq!(stats.membership_count, 3);
    }

    #[test]
    fn session_stats_mixed_message_types() {
        // 07_revenue.md: superchat/membership/textが混在するとき各カウントが正しいこと
        // L615, L616, L622 の全mutantを同時に殺す
        let messages = vec![
            make_export_message("sc1", "UC_a", "superchat", Some(SuperChatTier::Red)),
            make_export_message("sc2", "UC_b", "superchat", Some(SuperChatTier::Yellow)),
            make_export_message("m1", "UC_c", "membership", None),
            make_export_message("t1", "UC_d", "text", None),
            make_export_message("t2", "UC_d", "text", None), // 同一ユーザーの重複
        ];

        let stats = calculate_session_statistics(&messages, GiftStats::default());

        assert_eq!(stats.super_chat_count, 2);
        assert_eq!(stats.membership_count, 1);
        assert_eq!(stats.total_messages, 5);
        assert_eq!(stats.unique_viewers, 4); // UC_dは1人
    }

    // ========================================================================
    // SuperChatTier ordering (07_revenue.md: Blue < ... < Red)
    // ========================================================================

    #[test]
    fn tier_ordering() {
        assert!(SuperChatTier::Blue < SuperChatTier::Cyan);
        assert!(SuperChatTier::Cyan < SuperChatTier::Green);
        assert!(SuperChatTier::Green < SuperChatTier::Yellow);
        assert!(SuperChatTier::Yellow < SuperChatTier::Orange);
        assert!(SuperChatTier::Orange < SuperChatTier::Magenta);
        assert!(SuperChatTier::Magenta < SuperChatTier::Red);
    }

    // ========================================================================
    // compute_revenue_analytics (07_revenue.md: メッセージリストから集計)
    // ========================================================================

    /// テスト用ChatMessageヘルパー
    fn make_chat_message(
        channel_id: &str,
        author: &str,
        message_type: MessageType,
        metadata: Option<MessageMetadata>,
    ) -> ChatMessage {
        ChatMessage {
            id: format!("msg_{}", channel_id),
            channel_id: channel_id.to_string(),
            author: author.to_string(),
            message_type,
            metadata,
            ..Default::default()
        }
    }

    #[test]
    fn compute_revenue_analytics_empty_messages() {
        // 07_revenue.md: 空メッセージリスト → デフォルトのRevenueAnalytics
        let analytics = compute_revenue_analytics(&[]);

        assert_eq!(analytics.super_chat_count, 0);
        assert_eq!(analytics.super_sticker_count, 0);
        assert_eq!(analytics.membership_gains, 0);
        assert_eq!(analytics.super_chat_by_tier.total(), 0);
        assert!(analytics.top_contributors.is_empty());
        assert!(analytics.hourly_stats.is_empty());
    }

    #[test]
    fn compute_revenue_analytics_mixed_types() {
        // 07_revenue.md: SuperChat×2 + SuperSticker×1 + Membership×1 → 正しい集計
        let messages = vec![
            make_chat_message(
                "UC_a",
                "UserA",
                MessageType::SuperChat {
                    amount: "$10.00".to_string(),
                },
                Some(MessageMetadata {
                    superchat_colors: Some(SuperChatColors {
                        header_background: "#ffb300".to_string(), // Yellow tier
                        header_text: "#000000".to_string(),
                        body_background: "#ffb300".to_string(),
                        body_text: "#000000".to_string(),
                    }),
                    amount: Some("$10.00".to_string()),
                    badges: vec![],
                    badge_info: vec![],
                    color: None,
                    is_moderator: false,
                    is_verified: false,
                }),
            ),
            make_chat_message(
                "UC_b",
                "UserB",
                MessageType::SuperChat {
                    amount: "$200.00".to_string(),
                },
                Some(MessageMetadata {
                    superchat_colors: Some(SuperChatColors {
                        header_background: "#D00000".to_string(), // Red tier
                        header_text: "#ffffff".to_string(),
                        body_background: "#e62117".to_string(),
                        body_text: "#ffffff".to_string(),
                    }),
                    amount: Some("$200.00".to_string()),
                    badges: vec![],
                    badge_info: vec![],
                    color: None,
                    is_moderator: false,
                    is_verified: false,
                }),
            ),
            make_chat_message(
                "UC_c",
                "UserC",
                MessageType::SuperSticker {
                    amount: "$5.00".to_string(),
                },
                None,
            ),
            make_chat_message(
                "UC_d",
                "UserD",
                MessageType::Membership {
                    milestone_months: Some(6),
                },
                None,
            ),
        ];

        let analytics = compute_revenue_analytics(&messages);

        assert_eq!(analytics.super_chat_count, 2);
        assert_eq!(analytics.super_sticker_count, 1);
        assert_eq!(analytics.membership_gains, 1);
        assert_eq!(analytics.super_chat_by_tier.tier_yellow, 1);
        assert_eq!(analytics.super_chat_by_tier.tier_red, 1);
        assert_eq!(analytics.super_chat_by_tier.total(), 2);
        // 貢献者: UC_a(SC×1), UC_b(SC×1), UC_c(SS×1) = 3人
        assert_eq!(analytics.top_contributors.len(), 3);
    }

    #[test]
    fn compute_revenue_analytics_top_contributors_truncate() {
        // 07_revenue.md: 上位貢献者は10人にtruncateされる
        let messages: Vec<ChatMessage> = (0..15)
            .map(|i| {
                make_chat_message(
                    &format!("UC_{}", i),
                    &format!("User{}", i),
                    MessageType::SuperChat {
                        amount: "$5.00".to_string(),
                    },
                    None,
                )
            })
            .collect();

        let analytics = compute_revenue_analytics(&messages);

        assert_eq!(analytics.super_chat_count, 15);
        // 15人の貢献者がいるが上位10人にtruncateされる
        assert_eq!(analytics.top_contributors.len(), 10);
    }

    #[test]
    fn compute_revenue_analytics_contributors_sorted_by_count_then_tier() {
        // 07_revenue.md: SuperChat件数でソートし、同一件数の場合は最高tierで比較
        let messages = vec![
            // UC_a: SC×2, 最高tier=Red
            make_chat_message(
                "UC_a",
                "UserA",
                MessageType::SuperChat {
                    amount: "$200.00".to_string(),
                },
                Some(MessageMetadata {
                    superchat_colors: Some(SuperChatColors {
                        header_background: "#D00000".to_string(),
                        header_text: "#ffffff".to_string(),
                        body_background: "#e62117".to_string(),
                        body_text: "#ffffff".to_string(),
                    }),
                    amount: Some("$200.00".to_string()),
                    badges: vec![],
                    badge_info: vec![],
                    color: None,
                    is_moderator: false,
                    is_verified: false,
                }),
            ),
            make_chat_message(
                "UC_a",
                "UserA",
                MessageType::SuperChat {
                    amount: "$10.00".to_string(),
                },
                None,
            ),
            // UC_b: SC×2, 最高tier=Blue
            make_chat_message(
                "UC_b",
                "UserB",
                MessageType::SuperChat {
                    amount: "$1.00".to_string(),
                },
                None,
            ),
            make_chat_message(
                "UC_b",
                "UserB",
                MessageType::SuperChat {
                    amount: "$1.00".to_string(),
                },
                None,
            ),
            // UC_c: SC×1, 最高tier=Yellow
            make_chat_message(
                "UC_c",
                "UserC",
                MessageType::SuperChat {
                    amount: "$15.00".to_string(),
                },
                None,
            ),
        ];

        let analytics = compute_revenue_analytics(&messages);

        assert_eq!(analytics.top_contributors.len(), 3);
        // UC_a(2件, Red) と UC_b(2件, Blue) は同件数だがtierでUC_aが上
        assert_eq!(analytics.top_contributors[0].channel_id, "UC_a");
        assert_eq!(analytics.top_contributors[0].super_chat_count, 2);
        assert_eq!(analytics.top_contributors[1].channel_id, "UC_b");
        assert_eq!(analytics.top_contributors[1].super_chat_count, 2);
        // UC_c(1件) は最後
        assert_eq!(analytics.top_contributors[2].channel_id, "UC_c");
        assert_eq!(analytics.top_contributors[2].super_chat_count, 1);
    }

    #[test]
    fn compute_revenue_analytics_membership_gift_counted() {
        // 07_revenue.md: MembershipGiftもmembership_gainsにカウントされる
        let messages = vec![make_chat_message(
            "UC_a",
            "UserA",
            MessageType::MembershipGift { gift_count: 5 },
            None,
        )];

        let analytics = compute_revenue_analytics(&messages);

        assert_eq!(analytics.membership_gains, 1);
    }

    #[test]
    fn compute_revenue_analytics_tier_escalation() {
        // 同一コントリビューターがBlue(低tier)→Red(高tier)の順で送信した場合、
        // 最高tierはRedに更新されること
        let messages = vec![
            // 1件目: Blue tier（メタデータなし = デフォルトBlue）
            make_chat_message(
                "UC_x",
                "UserX",
                MessageType::SuperChat {
                    amount: "$1.00".to_string(),
                },
                None,
            ),
            // 2件目: Red tier
            make_chat_message(
                "UC_x",
                "UserX",
                MessageType::SuperChat {
                    amount: "$200.00".to_string(),
                },
                Some(MessageMetadata {
                    superchat_colors: Some(SuperChatColors {
                        header_background: "#D00000".to_string(),
                        header_text: "#ffffff".to_string(),
                        body_background: "#e62117".to_string(),
                        body_text: "#ffffff".to_string(),
                    }),
                    amount: Some("$200.00".to_string()),
                    badges: vec![],
                    badge_info: vec![],
                    color: None,
                    is_moderator: false,
                    is_verified: false,
                }),
            ),
        ];

        let analytics = compute_revenue_analytics(&messages);
        assert_eq!(analytics.top_contributors.len(), 1);
        assert_eq!(
            analytics.top_contributors[0].highest_tier,
            Some(SuperChatTier::Red)
        );
    }

    #[test]
    fn compute_revenue_analytics_supersticker_contributor_count() {
        // SuperStickerもcontributor件数にカウントされること
        let messages = vec![make_chat_message(
            "UC_s",
            "StickerUser",
            MessageType::SuperSticker {
                amount: "$5.00".to_string(),
            },
            None,
        )];

        let analytics = compute_revenue_analytics(&messages);
        assert_eq!(analytics.top_contributors.len(), 1);
        assert_eq!(analytics.top_contributors[0].super_chat_count, 1);
    }

    // ========================================================================
    // compute_session_analytics_from_rows (07_revenue.md: DB行データから集計)
    // ========================================================================

    #[test]
    fn compute_session_analytics_from_rows_mixed() {
        // 07_revenue.md: 種類と色ごとの件数から集計する
        let rows = vec![
            ("superchat".to_string(), Some("#FFB300".to_string()), 1), // Yellow
            ("superchat".to_string(), Some("#D00000".to_string()), 1), // Red
            ("supersticker".to_string(), None, 1),
            ("membership".to_string(), None, 1),
            ("membership_gift".to_string(), None, 1),
            ("text".to_string(), None, 1),
        ];

        let analytics = compute_session_analytics_from_rows(&rows);

        assert_eq!(analytics.super_chat_count, 2);
        assert_eq!(analytics.super_sticker_count, 1);
        assert_eq!(analytics.membership_gains, 2); // membership + membership_gift
        assert_eq!(analytics.super_chat_by_tier.tier_yellow, 1);
        assert_eq!(analytics.super_chat_by_tier.tier_red, 1);
        assert_eq!(analytics.super_chat_by_tier.total(), 2);
        // DB行ベースの集計では貢献者トラッキングはしない
        assert!(analytics.top_contributors.is_empty());
    }

    #[test]
    fn compute_session_analytics_from_rows_counts_each_row() {
        let rows = vec![("superchat".to_string(), Some("#00BFA5".to_string()), 3)];
        let analytics = compute_session_analytics_from_rows(&rows);
        assert_eq!(analytics.super_chat_count, 3);
        assert_eq!(analytics.super_chat_by_tier.tier_green, 3);
    }

    #[test]
    fn compute_session_analytics_from_rows_empty() {
        // 07_revenue.md: 空の行リスト → デフォルトのRevenueAnalytics
        let analytics = compute_session_analytics_from_rows(&[]);

        assert_eq!(analytics.super_chat_count, 0);
        assert_eq!(analytics.super_sticker_count, 0);
        assert_eq!(analytics.membership_gains, 0);
        assert_eq!(analytics.super_chat_by_tier.total(), 0);
    }

    #[test]
    fn compute_session_analytics_from_rows_superchat_without_color_is_unknown() {
        // 07_revenue.md: 色を保存する前の過去セッションのスーパーチャット → 段階不明（金額から推定しない）
        let rows = vec![("superchat".to_string(), None, 2)];

        let analytics = compute_session_analytics_from_rows(&rows);

        assert_eq!(analytics.super_chat_count, 2);
        assert_eq!(analytics.super_chat_by_tier.tier_unknown, 2);
        assert_eq!(analytics.super_chat_by_tier.tier_blue, 0);
    }

    // ========================================================================
    // convert_messages_to_export (07_revenue.md: ChatMessage→ExportMessage変換)
    // ========================================================================

    #[test]
    fn convert_messages_to_export_text() {
        // 07_revenue.md: TextメッセージはExportMessageのmessage_type="text"に変換
        let messages = vec![ChatMessage {
            id: "msg1".to_string(),
            timestamp: "2025-01-14T14:00:00Z".to_string(),
            author: "TestUser".to_string(),
            channel_id: "UC_test".to_string(),
            content: "Hello".to_string(),
            message_type: MessageType::Text,
            is_member: false,
            ..Default::default()
        }];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].message_type, "text");
        assert_eq!(exports[0].id, "msg1");
        assert_eq!(exports[0].author, "TestUser");
        assert_eq!(exports[0].author_id, "UC_test");
        assert_eq!(exports[0].content, "Hello");
        assert!(exports[0].amount_display.is_none());
        assert!(exports[0].tier.is_none());
        // metadata が無ければバッジは不明（08_database.md）
        assert_eq!(exports[0].is_moderator, None);
        assert!(!exports[0].is_member);
        assert_eq!(exports[0].is_verified, None);
    }

    #[test]
    fn convert_messages_to_export_superchat_with_color() {
        // 07_revenue.md: SuperChatは色情報からtierを判定し、amountとtierを含む
        let messages = vec![ChatMessage {
            id: "sc1".to_string(),
            timestamp: "2025-01-14T14:01:00Z".to_string(),
            author: "SCUser".to_string(),
            channel_id: "UC_sc".to_string(),
            content: "Super!".to_string(),
            message_type: MessageType::SuperChat {
                amount: "$50.00".to_string(),
            },
            metadata: Some(MessageMetadata {
                superchat_colors: Some(SuperChatColors {
                    header_background: "#C2185B".to_string(), // Magenta
                    header_text: "#ffffff".to_string(),
                    body_background: "#e91e63".to_string(),
                    body_text: "#ffffff".to_string(),
                }),
                amount: Some("$50.00".to_string()),
                badges: vec!["member".to_string()],
                badge_info: vec![],
                color: None,
                is_moderator: true,
                is_verified: false,
            }),
            is_member: true,
            ..Default::default()
        }];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].message_type, "superchat");
        assert_eq!(exports[0].amount_display, Some("$50.00".to_string()));
        assert_eq!(exports[0].tier, Some(SuperChatTier::Magenta));
        assert_eq!(exports[0].is_moderator, Some(true));
        assert!(exports[0].is_member);
        assert_eq!(exports[0].badges, Some(vec!["member".to_string()]));
    }

    #[test]
    fn convert_messages_to_export_supersticker() {
        // 07_revenue.md: SuperStickerはtierなし、amountあり
        let messages = vec![ChatMessage {
            id: "ss1".to_string(),
            message_type: MessageType::SuperSticker {
                amount: "$5.00".to_string(),
            },
            ..Default::default()
        }];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports[0].message_type, "supersticker");
        assert_eq!(exports[0].amount_display, Some("$5.00".to_string()));
        assert!(exports[0].tier.is_none());
    }

    #[test]
    fn convert_messages_to_export_membership() {
        // 07_revenue.md: Membershipはmessage_type="membership"
        let messages = vec![ChatMessage {
            id: "m1".to_string(),
            message_type: MessageType::Membership {
                milestone_months: Some(12),
            },
            ..Default::default()
        }];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports[0].message_type, "membership");
        assert!(exports[0].amount_display.is_none());
        assert!(exports[0].tier.is_none());
    }

    #[test]
    fn convert_messages_to_export_membership_gift() {
        // 07_revenue.md: MembershipGiftはmessage_type="membership_gift"
        let messages = vec![ChatMessage {
            id: "mg1".to_string(),
            message_type: MessageType::MembershipGift { gift_count: 5 },
            ..Default::default()
        }];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports[0].message_type, "membership_gift");
    }

    #[test]
    fn convert_messages_to_export_system() {
        // 07_revenue.md: Systemはmessage_type="system"
        let messages = vec![ChatMessage {
            id: "sys1".to_string(),
            message_type: MessageType::System,
            ..Default::default()
        }];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports[0].message_type, "system");
        assert!(exports[0].amount_display.is_none());
        assert!(exports[0].tier.is_none());
    }

    #[test]
    fn convert_messages_to_export_all_types() {
        // 07_revenue.md: 全MessageTypeを含むリストが正しく変換される
        let messages = vec![
            ChatMessage {
                id: "1".to_string(),
                message_type: MessageType::Text,
                ..Default::default()
            },
            ChatMessage {
                id: "2".to_string(),
                message_type: MessageType::SuperChat {
                    amount: "$10.00".to_string(),
                },
                ..Default::default()
            },
            ChatMessage {
                id: "3".to_string(),
                message_type: MessageType::SuperSticker {
                    amount: "$3.00".to_string(),
                },
                ..Default::default()
            },
            ChatMessage {
                id: "4".to_string(),
                message_type: MessageType::Membership {
                    milestone_months: None,
                },
                ..Default::default()
            },
            ChatMessage {
                id: "5".to_string(),
                message_type: MessageType::MembershipGift { gift_count: 1 },
                ..Default::default()
            },
            ChatMessage {
                id: "6".to_string(),
                message_type: MessageType::System,
                ..Default::default()
            },
        ];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports.len(), 6);
        assert_eq!(exports[0].message_type, "text");
        assert_eq!(exports[1].message_type, "superchat");
        assert_eq!(exports[2].message_type, "supersticker");
        assert_eq!(exports[3].message_type, "membership");
        assert_eq!(exports[4].message_type, "membership_gift");
        assert_eq!(exports[5].message_type, "system");
    }

    // ========================================================================
    // 追加テスト: 残存missed mutantsを殺す
    // ========================================================================

    #[test]
    fn compute_revenue_analytics_supersticker_multiple_count() {
        // 07_revenue.md: 同一コントリビューターが複数SuperStickerを送信した場合、
        // contributor件数が正しく加算されること
        // 対象mutant: L278 `entry.1 += 1` → `*= 1`
        // (*= 1 の場合、2件目以降で count が 1×1=1 のままになる)
        let messages = vec![
            make_chat_message(
                "UC_s",
                "StickerUser",
                MessageType::SuperSticker {
                    amount: "$5.00".to_string(),
                },
                None,
            ),
            make_chat_message(
                "UC_s",
                "StickerUser",
                MessageType::SuperSticker {
                    amount: "$5.00".to_string(),
                },
                None,
            ),
            make_chat_message(
                "UC_s",
                "StickerUser",
                MessageType::SuperSticker {
                    amount: "$5.00".to_string(),
                },
                None,
            ),
        ];

        let analytics = compute_revenue_analytics(&messages);

        // 3件のSuperStickerで件数は3になるべき (*= 1 mutantでは1になる)
        assert_eq!(analytics.top_contributors.len(), 1);
        assert_eq!(analytics.top_contributors[0].super_chat_count, 3);
    }

    #[test]
    fn export_to_csv_without_metadata_data_rows_present() {
        // 07_revenue.md: include_metadata=false のCSVにはメタデータヘッダなし、
        // データ行は正しく出力されること
        // 対象mutant: export_to_csv内のinclude_metadataブランチ
        let data = make_test_export_data();
        let config = ExportConfig {
            format: "csv".to_string(),
            include_metadata: false,
            include_system_messages: false,
            max_records: None,
            sort_order: None,
        };

        let csv = export_to_csv(&data, &config).unwrap();

        // メタデータヘッダは含まれない
        assert!(!csv.contains("# Metadata"));
        assert!(!csv.contains("# Session ID"));
        // カラムヘッダから始まる
        assert!(csv.starts_with("id,timestamp,author"));
        // データ行が含まれる (msg1, msg2 の両方)
        assert!(csv.contains("\"msg1\""));
        assert!(csv.contains("\"msg2\""));
        // SuperChatのtier情報が含まれる
        assert!(csv.contains("yellow"));
        assert!(csv.contains("$10.00"));
    }

    #[test]
    fn export_to_json_messages_only_content_verified() {
        // 07_revenue.md: include_metadata=false のJSONはmessagesの配列のみ返し、
        // 各要素のフィールドが正しいこと
        // 対象mutant: export_to_json内のinclude_metadataブランチ
        let data = make_test_export_data();
        let config = ExportConfig {
            format: "json".to_string(),
            include_metadata: false,
            include_system_messages: false,
            max_records: None,
            sort_order: None,
        };

        let json = export_to_json(&data, &config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // トップレベルは配列
        assert!(parsed.is_array());
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        // metadata/statistics フィールドは含まれない (配列なので存在しない)
        assert!(parsed.get("metadata").is_none());
        assert!(parsed.get("statistics").is_none());

        // 各メッセージのフィールドを検証
        let first = &arr[0];
        assert_eq!(first["id"], "msg1");
        assert_eq!(first["message_type"], "text");

        let second = &arr[1];
        assert_eq!(second["id"], "msg2");
        assert_eq!(second["message_type"], "superchat");
        assert_eq!(second["amount_display"], "$10.00");
    }

    // ========================================================================
    // ギフト集計 (07_revenue.md: ギフト集計)
    // ========================================================================

    fn gift(name: &str, jewels: Option<u32>) -> GiftDetails {
        GiftDetails {
            gift_name: name.to_string(),
            gift_image_url: Some(format!("https://example.com/{}.png", name)),
            jewel_count: jewels,
        }
    }

    #[test]
    fn gift_stats_empty() {
        let stats = GiftStats::from_gifts(&[] as &[GiftDetails]);
        assert_eq!(stats.gift_count, 0);
        assert!(stats.gifts_by_name.is_empty());
        assert_eq!(stats.jewel_known_count, 0);
        assert_eq!(stats.total_jewels, 0);
    }

    #[test]
    fn gift_stats_spec_example() {
        // Hiding(不明)・Press F(10)・Press F(10) → 3件、Press F 2件・Hiding 1件、判明 2件、合計 20
        let stats = GiftStats::from_gifts(&[
            gift("Hiding", None),
            gift("Press F", Some(10)),
            gift("Press F", Some(10)),
        ]);
        assert_eq!(stats.gift_count, 3);
        let by_name: Vec<(&str, usize)> = stats
            .gifts_by_name
            .iter()
            .map(|g| (g.gift_name.as_str(), g.count))
            .collect();
        assert_eq!(by_name, vec![("Press F", 2), ("Hiding", 1)]);
        assert_eq!(
            stats.gifts_by_name[0].gift_image_url.as_deref(),
            Some("https://example.com/Press F.png")
        );
        assert_eq!(stats.jewel_known_count, 2);
        assert_eq!(stats.total_jewels, 20);
    }

    #[test]
    fn gift_stats_same_count_sorted_by_name() {
        let stats = GiftStats::from_gifts(&[gift("Zebra", None), gift("Apple", None)]);
        let names: Vec<&str> = stats
            .gifts_by_name
            .iter()
            .map(|g| g.gift_name.as_str())
            .collect();
        assert_eq!(names, vec!["Apple", "Zebra"]);
    }

    #[test]
    fn compute_revenue_analytics_counts_gifts_separately() {
        let messages = vec![
            make_chat_message("", "@a", MessageType::Gift(gift("Hiding", None)), None),
            make_chat_message(
                "UC_b",
                "@b",
                MessageType::Gift(gift("Press F", Some(10))),
                None,
            ),
            make_chat_message(
                "UC_c",
                "@c",
                MessageType::SuperChat {
                    amount: "$10.00".to_string(),
                },
                None,
            ),
        ];
        let analytics = compute_revenue_analytics(&messages);
        assert_eq!(analytics.gifts.gift_count, 2);
        assert_eq!(analytics.gifts.total_jewels, 10);
        // 既存の集計には影響しない
        assert_eq!(analytics.super_chat_count, 1);
        assert_eq!(analytics.membership_gains, 0);
        assert_eq!(analytics.top_contributors.len(), 1);
    }

    #[test]
    fn gift_stats_from_db_metadata() {
        // 08_database.md: gift 行の metadata JSON から集計する。壊れた JSON は数えない
        let rows = vec![
            Some(r#"{"gift_name":"Press F","gift_image_url":null,"jewel_count":10}"#.to_string()),
            Some(r#"{"gift_name":"Hiding","gift_image_url":null,"jewel_count":null}"#.to_string()),
            Some("not json".to_string()),
            None,
        ];
        let stats = gift_stats_from_metadata(&rows);
        assert_eq!(stats.gift_count, 2);
        assert_eq!(stats.jewel_known_count, 1);
        assert_eq!(stats.total_jewels, 10);
    }

    #[test]
    fn convert_messages_to_export_gift() {
        // 07_revenue.md: message_type = gift、amount_display = "10 Jewels"（不明なら空）
        let messages = vec![
            make_chat_message(
                "UC_b",
                "@b",
                MessageType::Gift(gift("Press F", Some(10))),
                None,
            ),
            make_chat_message("", "@a", MessageType::Gift(gift("Hiding", None)), None),
        ];
        let exported = convert_messages_to_export(&messages, "s", "UC_own");
        assert_eq!(exported[0].message_type, "gift");
        assert_eq!(exported[0].amount_display.as_deref(), Some("10 Jewels"));
        assert_eq!(exported[1].message_type, "gift");
        assert_eq!(exported[1].amount_display, None);
    }

    #[test]
    fn json_export_statistics_include_gifts() {
        let mut data = make_test_export_data();
        data.statistics.gifts = GiftStats::from_gifts(&[gift("Press F", Some(10))]);
        let config = ExportConfig {
            format: "json".to_string(),
            include_metadata: true,
            include_system_messages: true,
            max_records: None,
            sort_order: None,
        };
        let json: serde_json::Value =
            serde_json::from_str(&export_to_json(&data, &config).unwrap()).unwrap();
        assert_eq!(json["statistics"]["gifts"]["gift_count"], 1);
        assert_eq!(json["statistics"]["gifts"]["total_jewels"], 10);
    }

    // ========================================================================
    // 過去セッションの分析・エクスポートを実際の DB で通す
    // (07_revenue.md / 08_database.md。SQL の列名の誤りは純粋関数のテストでは見つからない)
    // ========================================================================

    mod with_db {
        use super::*;
        use crate::core::{BadgeInfo, MessageMetadata, SuperChatColors};
        use crate::database::{self, Database};

        fn superchat(id: &str, header: &str) -> ChatMessage {
            ChatMessage {
                id: id.to_string(),
                timestamp: "2026-09-28T12:00:00Z".to_string(),
                timestamp_usec: "1790000000000000".to_string(),
                author: "@donor".to_string(),
                channel_id: "UC_donor".to_string(),
                content: "応援してます".to_string(),
                message_type: MessageType::SuperChat {
                    amount: "¥500".to_string(),
                },
                metadata: Some(MessageMetadata {
                    amount: Some("¥500".to_string()),
                    superchat_colors: Some(SuperChatColors {
                        header_background: header.to_string(),
                        header_text: "#FFFFFF".to_string(),
                        body_background: header.to_string(),
                        body_text: "#FFFFFF".to_string(),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }
        }

        fn moderator_text(id: &str) -> ChatMessage {
            ChatMessage {
                id: id.to_string(),
                timestamp: "2026-09-28T12:00:01Z".to_string(),
                timestamp_usec: "1790000001000000".to_string(),
                author: "@mod".to_string(),
                channel_id: "UC_mod".to_string(),
                content: "モデレーターです".to_string(),
                message_type: MessageType::Text,
                metadata: Some(MessageMetadata {
                    badges: vec!["moderator".to_string()],
                    badge_info: vec![BadgeInfo {
                        badge_type: "moderator".to_string(),
                        label: "Moderator".to_string(),
                        tooltip: Some("Moderator".to_string()),
                        icon_url: None,
                    }],
                    is_moderator: true,
                    ..Default::default()
                }),
                ..Default::default()
            }
        }

        fn gift(id: &str) -> ChatMessage {
            ChatMessage {
                id: id.to_string(),
                timestamp: "2026-09-28T12:00:02Z".to_string(),
                timestamp_usec: "1790000002000000".to_string(),
                author: "@gifter".to_string(),
                content: "Press F".to_string(),
                message_type: MessageType::Gift(GiftDetails {
                    gift_name: "Press F".to_string(),
                    gift_image_url: None,
                    jewel_count: Some(10),
                }),
                ..Default::default()
            }
        }

        /// 005 より前に保存した行を再現する（色とバッジを NULL に戻す）
        fn forget_columns(conn: &rusqlite::Connection, message_id: &str) {
            conn.execute(
                "UPDATE messages SET superchat_color = NULL, is_moderator = NULL,
                        is_verified = NULL, badges = NULL WHERE message_id = ?1",
                [message_id],
            )
            .unwrap();
        }

        async fn session_with(messages: &[ChatMessage], old: &[&str]) -> (Database, String) {
            let db = Database::new_in_memory().unwrap();
            let session_id = {
                let conn = db.connection().await;
                let session_id =
                    database::create_session(&conn, None, Some("配信"), None, None).unwrap();
                for message in messages {
                    database::save_message(&conn, &session_id, None, message, None).unwrap();
                }
                for id in old {
                    forget_columns(&conn, id);
                }
                session_id
            };
            (db, session_id)
        }

        fn export_config() -> ExportConfig {
            ExportConfig {
                format: "json".to_string(),
                include_metadata: true,
                include_system_messages: true,
                max_records: None,
                sort_order: None,
            }
        }

        #[tokio::test]
        async fn session_analytics_counts_saved_color_and_old_rows_as_unknown() {
            // 色を保存した緑のスパチャ 1 件 + 色を保存する前のスパチャ 1 件
            let messages = [
                superchat("sc_new", "#00BFA5"),
                superchat("sc_old", "#00BFA5"),
                gift("g1"),
            ];
            let (db, session_id) = session_with(&messages, &["sc_old"]).await;
            let conn = db.connection().await;

            let analytics = session_analytics(&conn, &session_id).unwrap();

            assert_eq!(analytics.super_chat_count, 2);
            assert_eq!(analytics.super_chat_by_tier.tier_green, 1);
            assert_eq!(analytics.super_chat_by_tier.tier_unknown, 1);
            assert_eq!(analytics.gifts.gift_count, 1);
        }

        #[tokio::test]
        async fn session_export_reads_saved_badges_and_leaves_unknown_as_none() {
            let messages = [
                moderator_text("mod_new"),
                moderator_text("mod_old"),
                superchat("sc1", "#C2185B"),
                gift("g1"),
            ];
            let (db, session_id) = session_with(&messages, &["mod_old"]).await;
            let conn = db.connection().await;

            let data = session_export_data(&conn, &session_id, &export_config()).unwrap();
            let by_id = |id: &str| data.messages.iter().find(|m| m.id == id).unwrap();

            // id は YouTube のメッセージ ID
            assert_eq!(data.messages.len(), 4);
            let saved = by_id("mod_new");
            assert_eq!(saved.is_moderator, Some(true));
            assert_eq!(saved.is_verified, Some(false));
            assert_eq!(saved.badges, Some(vec!["moderator".to_string()]));
            // 005 より前の行は不明
            let old = by_id("mod_old");
            assert_eq!(old.is_moderator, None);
            assert_eq!(old.badges, None);
            // バッジを読めない種類（ギフト）は不明
            assert_eq!(by_id("g1").is_moderator, None);
            assert_eq!(by_id("g1").amount_display.as_deref(), Some("10 Jewels"));
            // 段階は保存した色から
            assert_eq!(by_id("sc1").tier, Some(SuperChatTier::Magenta));
        }

        #[tokio::test]
        async fn session_export_csv_leaves_unknown_blank() {
            let (db, session_id) = session_with(&[moderator_text("mod_old")], &["mod_old"]).await;
            let conn = db.connection().await;
            let config = ExportConfig {
                format: "csv".to_string(),
                include_metadata: false,
                ..export_config()
            };

            let data = session_export_data(&conn, &session_id, &config).unwrap();
            let csv = export_to_csv(&data, &config).unwrap();
            let row = csv.lines().nth(1).unwrap();

            // ...,tier,is_moderator,is_member,is_verified,badges
            assert!(row.ends_with(r#","",,false,,"""#), "{row}");
        }
    }
}
