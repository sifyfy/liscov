//! Analytics and export commands
//!
//! Implements 07_revenue.md specification
//! Note: SuperChat amounts are NOT calculated numerically due to different currencies.
//! Instead, we use tier-based aggregation based on YouTube's color scheme.

use crate::core::{ChatMessage, MessageType};
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
    Blue,     // Lowest tier (USD $1-2)
    Cyan,     // USD $2-5
    Green,    // USD $5-10
    Yellow,   // USD $10-20
    Orange,   // USD $20-50
    Magenta,  // USD $50-100
    Red,      // Highest tier (USD $100-500)
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
}

impl SuperChatTierStats {
    pub fn increment(&mut self, tier: SuperChatTier) {
        match tier {
            SuperChatTier::Red => self.tier_red += 1,
            SuperChatTier::Magenta => self.tier_magenta += 1,
            SuperChatTier::Orange => self.tier_orange += 1,
            SuperChatTier::Yellow => self.tier_yellow += 1,
            SuperChatTier::Green => self.tier_green += 1,
            SuperChatTier::Cyan => self.tier_cyan += 1,
            SuperChatTier::Blue => self.tier_blue += 1,
        }
    }

    pub fn total(&self) -> usize {
        self.tier_red + self.tier_magenta + self.tier_orange +
        self.tier_yellow + self.tier_green + self.tier_cyan + self.tier_blue
    }
}

/// Revenue analytics data (07_revenue.md)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct RevenueAnalytics {
    pub super_chat_count: usize,
    pub super_chat_by_tier: SuperChatTierStats,
    pub super_sticker_count: usize,
    pub membership_gains: usize,
    pub hourly_stats: Vec<HourlyStats>,
    pub top_contributors: Vec<ContributorInfo>,
}

impl Default for RevenueAnalytics {
    fn default() -> Self {
        Self {
            super_chat_count: 0,
            super_chat_by_tier: SuperChatTierStats::default(),
            super_sticker_count: 0,
            membership_gains: 0,
            hourly_stats: vec![],
            top_contributors: vec![],
        }
    }
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
    pub tier: Option<SuperChatTier>,
    pub is_moderator: bool,
    pub is_member: bool,
    pub is_verified: bool,
    pub badges: Vec<String>,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatistics {
    pub total_messages: usize,
    pub unique_viewers: usize,
    pub super_chat_count: usize,
    pub super_chat_by_tier: SuperChatTierStats,
    pub membership_count: usize,
}

/// Determine SuperChat tier from header_background_color
/// YouTube uses specific colors for different tier levels
fn determine_tier_from_color(header_color: &str) -> SuperChatTier {
    // Common YouTube SuperChat header background colors (hex without #)
    // These values may need adjustment based on actual YouTube API responses
    let color = header_color.to_lowercase().replace('#', "");

    // Try to parse as hex color and determine tier
    // YouTube uses specific color ranges for tiers
    match color.as_str() {
        // Orange tier (check before Red to avoid starts_with("e6") false positive on e65100)
        c if c.contains("ff5722") || c.contains("e65100") || c.contains("f57c00") => SuperChatTier::Orange,
        // Red tier (highest)
        c if c.contains("e62117") || c.contains("ff0000") || c.starts_with("e6") => SuperChatTier::Red,
        // Magenta tier
        c if c.contains("e91e63") || c.contains("c2185b") => SuperChatTier::Magenta,
        // Yellow tier
        c if c.contains("ffb300") || c.contains("ffca28") || c.contains("ffc107") => SuperChatTier::Yellow,
        // Green tier
        c if c.contains("00e676") || c.contains("1de9b6") || c.contains("00c853") => SuperChatTier::Green,
        // Cyan tier
        c if c.contains("00bcd4") || c.contains("00b8d4") || c.contains("00acc1") => SuperChatTier::Cyan,
        // Blue tier (lowest) - default for unrecognized colors
        _ => SuperChatTier::Blue,
    }
}

/// Determine tier from amount string as fallback
fn determine_tier_from_amount(amount: &str) -> SuperChatTier {
    // This is a fallback when color info is not available
    // Parse the numeric value and estimate tier based on common ranges
    let value = parse_amount_value(amount).unwrap_or(0.0);

    // These are rough estimates based on USD equivalent
    // Real tier determination should use color from YouTube API
    if value >= 100.0 {
        SuperChatTier::Red
    } else if value >= 50.0 {
        SuperChatTier::Magenta
    } else if value >= 20.0 {
        SuperChatTier::Orange
    } else if value >= 10.0 {
        SuperChatTier::Yellow
    } else if value >= 5.0 {
        SuperChatTier::Green
    } else if value >= 2.0 {
        SuperChatTier::Cyan
    } else {
        SuperChatTier::Blue
    }
}

fn parse_amount_value(amount_str: &str) -> Option<f64> {
    if amount_str.is_empty() {
        return None;
    }

    let clean_amount: String = amount_str
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();

    clean_amount.parse::<f64>().ok()
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
            MessageType::SuperChat { amount } => {
                analytics.super_chat_count += 1;

                // 色情報があればそこからtierを判定、なければ金額からフォールバック
                let tier = if let Some(ref metadata) = message.metadata {
                    if let Some(ref colors) = metadata.superchat_colors {
                        determine_tier_from_color(&colors.header_background)
                    } else {
                        determine_tier_from_amount(amount)
                    }
                } else {
                    determine_tier_from_amount(amount)
                };

                analytics.super_chat_by_tier.increment(tier);

                // 貢献者情報を更新
                let entry = contributors
                    .entry(message.channel_id.clone())
                    .or_insert((message.author.clone(), 0, None));
                entry.1 += 1;
                // より高いtierがあれば更新
                if entry.2.is_none() || tier > entry.2.unwrap() {
                    entry.2 = Some(tier);
                }
            }
            MessageType::SuperSticker { amount: _ } => {
                analytics.super_sticker_count += 1;

                // SuperStickerは件数カウントのみ（tier統計には影響しない）
                let entry = contributors
                    .entry(message.channel_id.clone())
                    .or_insert((message.author.clone(), 0, None));
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
        .map(|(channel_id, (display_name, super_chat_count, highest_tier))| {
            ContributorInfo {
                channel_id,
                display_name,
                super_chat_count,
                highest_tier,
            }
        })
        .collect();

    contributors_vec.sort_by(|a, b| {
        match b.super_chat_count.cmp(&a.super_chat_count) {
            std::cmp::Ordering::Equal => b.highest_tier.cmp(&a.highest_tier),
            other => other,
        }
    });

    contributors_vec.truncate(10);
    analytics.top_contributors = contributors_vec;

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

/// DB行データからRevenueAnalyticsを計算する純粋関数
///
/// 各行は (message_type, amount, header_color) のタプル
pub(crate) fn compute_session_analytics_from_rows(
    rows: &[(String, Option<String>, Option<String>)],
) -> RevenueAnalytics {
    let mut analytics = RevenueAnalytics::default();

    for (message_type, amount_str, header_color) in rows {
        match message_type.as_str() {
            "superchat" => {
                analytics.super_chat_count += 1;

                let tier = if let Some(color) = header_color {
                    determine_tier_from_color(color)
                } else if let Some(amt) = amount_str {
                    determine_tier_from_amount(amt)
                } else {
                    SuperChatTier::Blue
                };

                analytics.super_chat_by_tier.increment(tier);
            }
            "supersticker" => {
                analytics.super_sticker_count += 1;
            }
            "membership" | "membership_gift" => {
                analytics.membership_gains += 1;
            }
            _ => {}
        }
    }

    analytics
}

/// Get analytics for a specific session from database
#[tauri::command]
pub async fn get_session_analytics(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<RevenueAnalytics, CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;

    // セッションのメッセージをカラー情報と一緒に取得
    let mut stmt = conn
        .prepare(
            "SELECT message_type, amount, header_color FROM messages WHERE session_id = ?"
        )
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?;

    let rows: Vec<(String, Option<String>, Option<String>)> = stmt
        .query_map([&session_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?;

    Ok(compute_session_analytics_from_rows(&rows))
}

/// Export session data to file
#[tauri::command]
pub async fn export_session_data(
    state: State<'_, AppState>,
    session_id: String,
    file_path: String,
    config: ExportConfig,
) -> Result<(), CommandError> {
    let db_guard = state.database.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| CommandError::DatabaseError("Database not initialized".to_string()))?;

    let conn = db.connection().await;

    // セッションメタデータを取得
    let session = conn
        .query_row(
            "SELECT id, start_time, end_time, stream_url, stream_title,
                    broadcaster_channel_id, broadcaster_name, total_messages, total_revenue
             FROM sessions WHERE id = ?",
            [&session_id],
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

    // メッセージを取得
    let limit_clause = config.max_records.map(|n| format!(" LIMIT {}", n)).unwrap_or_default();
    let query = format!(
        "SELECT id, timestamp, author, channel_id, content, message_type, amount, is_member,
                is_moderator, is_verified, badges, header_color
         FROM messages WHERE session_id = ? ORDER BY timestamp{}",
        limit_clause
    );

    let mut stmt = conn.prepare(&query)
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?;

    let messages: Vec<ExportMessage> = stmt
        .query_map([&session_id], |row| {
            let message_type: String = row.get(5)?;
            let amount: Option<String> = row.get(6)?;
            let header_color: Option<String> = row.get(11)?;
            let badges_json: Option<String> = row.get(10)?;

            let tier = if message_type == "superchat" {
                if let Some(ref color) = header_color {
                    Some(determine_tier_from_color(color))
                } else if let Some(ref amt) = amount {
                    Some(determine_tier_from_amount(amt))
                } else {
                    None
                }
            } else {
                None
            };

            let badges: Vec<String> = badges_json
                .and_then(|j| serde_json::from_str(&j).ok())
                .unwrap_or_default();

            Ok(ExportMessage {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                author: row.get(2)?,
                author_id: row.get(3)?,
                content: row.get(4)?,
                message_type,
                amount_display: amount,
                tier,
                is_member: row.get(7)?,
                is_moderator: row.get(8).unwrap_or(false),
                is_verified: row.get(9).unwrap_or(false),
                badges,
            })
        })
        .map_err(|e| CommandError::DatabaseError(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let statistics = calculate_session_statistics(&messages);

    let export_data = SessionExportData {
        metadata: session,
        messages,
        statistics,
    };

    // フォーマットに応じてエクスポート
    let content = match config.format.as_str() {
        "json" => export_to_json(&export_data, &config)?,
        "csv" => export_to_csv(&export_data, &config)?,
        _ => return Err(CommandError::InvalidInput(format!("Unsupported format: {}", config.format))),
    };

    // ファイルに書き出し
    let mut file = File::create(&file_path)
        .map_err(|e| CommandError::IoError(format!("Failed to create file: {}", e)))?;

    file.write_all(content.as_bytes())
        .map_err(|e| CommandError::IoError(format!("Failed to write file: {}", e)))?;

    Ok(())
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
                MessageType::SuperChat { amount } => {
                    let t = if let Some(ref metadata) = msg.metadata {
                        if let Some(ref colors) = metadata.superchat_colors {
                            determine_tier_from_color(&colors.header_background)
                        } else {
                            determine_tier_from_amount(amount)
                        }
                    } else {
                        determine_tier_from_amount(amount)
                    };
                    ("superchat".to_string(), Some(amount.clone()), Some(t))
                }
                MessageType::SuperSticker { amount } => {
                    ("supersticker".to_string(), Some(amount.clone()), None)
                }
                MessageType::Membership { .. } => ("membership".to_string(), None, None),
                MessageType::MembershipGift { .. } => ("membership_gift".to_string(), None, None),
                MessageType::System => ("system".to_string(), None, None),
            };

            let (is_moderator, is_verified, badges) = if let Some(ref metadata) = msg.metadata {
                (metadata.is_moderator, metadata.is_verified, metadata.badges.clone())
            } else {
                (false, false, vec![])
            };

            ExportMessage {
                id: msg.id.clone(),
                timestamp: msg.timestamp.clone(),
                author: msg.author.clone(),
                author_id: msg.channel_id.clone(),
                content: msg.content.clone(),
                message_type: message_type_str,
                amount_display,
                tier,
                is_moderator,
                is_member: msg.is_member,
                is_verified,
                badges,
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
    let messages = state.messages.read().await;

    // 多接続モデル: 全接続のセッションID・配信者IDを収集
    let (session_ids, broadcaster_ids): (Vec<String>, Vec<String>) = {
        let connections = state.connections.read().await;
        let sids: Vec<String> = connections.values()
            .filter_map(|c| c.session_id.clone())
            .collect();
        let bids: Vec<String> = connections.values()
            .map(|c| c.broadcaster_channel_id.clone())
            .filter(|id| !id.is_empty())
            .collect();
        (sids, bids)
    };
    // 後方互換: 単一値として最初の要素を使用（エクスポートヘッダ用）
    let session_id = session_ids.first().cloned().unwrap_or_else(|| "current".to_string());
    let broadcaster_id = broadcaster_ids.first().cloned().unwrap_or_default();

    // VecDequeをVecに変換して純粋関数に渡す
    let messages_vec: Vec<ChatMessage> = messages
        .iter()
        .take(config.max_records.unwrap_or(usize::MAX))
        .cloned()
        .collect();
    let export_messages = convert_messages_to_export(&messages_vec, &session_id, &broadcaster_id);

    let statistics = calculate_session_statistics(&export_messages);

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

    let content = match config.format.as_str() {
        "json" => export_to_json(&export_data, &config)?,
        "csv" => export_to_csv(&export_data, &config)?,
        _ => return Err(CommandError::InvalidInput(format!("Unsupported format: {}", config.format))),
    };

    let mut file = File::create(&file_path)
        .map_err(|e| CommandError::IoError(format!("Failed to create file: {}", e)))?;

    file.write_all(content.as_bytes())
        .map_err(|e| CommandError::IoError(format!("Failed to write file: {}", e)))?;

    Ok(())
}

// Helper functions

/// Calculate session statistics from export messages (DRY: used by both export functions)
fn calculate_session_statistics(messages: &[ExportMessage]) -> SessionStatistics {
    let mut unique_viewers: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut super_chat_count = 0;
    let mut super_chat_by_tier = SuperChatTierStats::default();
    let mut membership_count = 0;

    for msg in messages {
        unique_viewers.insert(msg.author_id.clone());

        match msg.message_type.as_str() {
            "superchat" => {
                super_chat_count += 1;
                if let Some(tier) = msg.tier {
                    super_chat_by_tier.increment(tier);
                }
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
        csv.push_str(&format!("# Total Messages,{}\n", data.statistics.total_messages));
        csv.push_str(&format!("# Unique Viewers,{}\n", data.statistics.unique_viewers));
        csv.push_str(&format!("# SuperChat Count,{}\n", data.statistics.super_chat_count));
        csv.push_str(&format!("# Export Time,{}\n", data.metadata.export_time));
        csv.push('\n');
    }

    // Header (per spec)
    csv.push_str("id,timestamp,author,author_id,content,message_type,amount_display,tier,is_moderator,is_member,is_verified,badges\n");

    // Data rows
    for msg in &data.messages {
        let amount_str = msg.amount_display.as_deref().unwrap_or("");
        let tier_str = msg.tier.map(|t| format!("{:?}", t).to_lowercase()).unwrap_or_default();
        let content_escaped = msg.content.replace('"', "\"\"");
        let badges_str = msg.badges.join(";");

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
            msg.is_moderator,
            msg.is_member,
            msg.is_verified,
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
    // determine_tier_from_color (07_revenue.md: Tier判定 - 色ベース)
    // ========================================================================

    #[test]
    fn tier_from_color_red() {
        assert_eq!(determine_tier_from_color("#e62117"), SuperChatTier::Red);
        assert_eq!(determine_tier_from_color("ff0000"), SuperChatTier::Red);
        assert_eq!(determine_tier_from_color("e6abcd"), SuperChatTier::Red); // starts_with("e6")
    }

    #[test]
    fn tier_from_color_magenta() {
        assert_eq!(determine_tier_from_color("e91e63"), SuperChatTier::Magenta);
        assert_eq!(determine_tier_from_color("#c2185b"), SuperChatTier::Magenta);
    }

    #[test]
    fn tier_from_color_orange() {
        assert_eq!(determine_tier_from_color("ff5722"), SuperChatTier::Orange);
        assert_eq!(determine_tier_from_color("e65100"), SuperChatTier::Orange);
        assert_eq!(determine_tier_from_color("f57c00"), SuperChatTier::Orange);
    }

    #[test]
    fn tier_from_color_yellow() {
        assert_eq!(determine_tier_from_color("ffb300"), SuperChatTier::Yellow);
        assert_eq!(determine_tier_from_color("ffca28"), SuperChatTier::Yellow);
        assert_eq!(determine_tier_from_color("ffc107"), SuperChatTier::Yellow);
    }

    #[test]
    fn tier_from_color_green() {
        assert_eq!(determine_tier_from_color("00e676"), SuperChatTier::Green);
        assert_eq!(determine_tier_from_color("1de9b6"), SuperChatTier::Green);
        assert_eq!(determine_tier_from_color("00c853"), SuperChatTier::Green);
    }

    #[test]
    fn tier_from_color_cyan() {
        assert_eq!(determine_tier_from_color("00bcd4"), SuperChatTier::Cyan);
        assert_eq!(determine_tier_from_color("00b8d4"), SuperChatTier::Cyan);
        assert_eq!(determine_tier_from_color("00acc1"), SuperChatTier::Cyan);
    }

    #[test]
    fn tier_from_color_blue_default() {
        assert_eq!(determine_tier_from_color("1565c0"), SuperChatTier::Blue);
        assert_eq!(determine_tier_from_color("unknown"), SuperChatTier::Blue);
        assert_eq!(determine_tier_from_color(""), SuperChatTier::Blue);
    }

    #[test]
    fn tier_from_color_case_insensitive() {
        assert_eq!(determine_tier_from_color("E62117"), SuperChatTier::Red);
        assert_eq!(determine_tier_from_color("#E91E63"), SuperChatTier::Magenta);
    }

    // ========================================================================
    // determine_tier_from_amount (07_revenue.md: Tier判定 - 金額ベース)
    // ========================================================================

    #[test]
    fn tier_from_amount_red() {
        assert_eq!(determine_tier_from_amount("$200.00"), SuperChatTier::Red);
        assert_eq!(determine_tier_from_amount("¥10000"), SuperChatTier::Red); // 10000 >= 100
    }

    #[test]
    fn tier_from_amount_magenta() {
        assert_eq!(determine_tier_from_amount("$75.00"), SuperChatTier::Magenta);
        assert_eq!(determine_tier_from_amount("$50.00"), SuperChatTier::Magenta);
    }

    #[test]
    fn tier_from_amount_orange() {
        assert_eq!(determine_tier_from_amount("$30.00"), SuperChatTier::Orange);
        assert_eq!(determine_tier_from_amount("$20.00"), SuperChatTier::Orange);
    }

    #[test]
    fn tier_from_amount_yellow() {
        assert_eq!(determine_tier_from_amount("$15.00"), SuperChatTier::Yellow);
        assert_eq!(determine_tier_from_amount("$10.00"), SuperChatTier::Yellow);
    }

    #[test]
    fn tier_from_amount_green() {
        assert_eq!(determine_tier_from_amount("$7.00"), SuperChatTier::Green);
        assert_eq!(determine_tier_from_amount("$5.00"), SuperChatTier::Green);
    }

    #[test]
    fn tier_from_amount_cyan() {
        assert_eq!(determine_tier_from_amount("$3.00"), SuperChatTier::Cyan);
        assert_eq!(determine_tier_from_amount("$2.00"), SuperChatTier::Cyan);
    }

    #[test]
    fn tier_from_amount_blue() {
        assert_eq!(determine_tier_from_amount("$1.00"), SuperChatTier::Blue);
        assert_eq!(determine_tier_from_amount("$0.50"), SuperChatTier::Blue);
    }

    #[test]
    fn tier_from_amount_unparseable() {
        assert_eq!(determine_tier_from_amount(""), SuperChatTier::Blue);
        assert_eq!(determine_tier_from_amount("free"), SuperChatTier::Blue);
    }

    // ========================================================================
    // parse_amount_value (07_revenue.md: 金額パース)
    // ========================================================================

    #[test]
    fn parse_amount_value_usd() {
        assert_eq!(parse_amount_value("$10.00"), Some(10.0));
    }

    #[test]
    fn parse_amount_value_yen() {
        assert_eq!(parse_amount_value("¥1000"), Some(1000.0));
    }

    #[test]
    fn parse_amount_value_euro() {
        // ',' is filtered out by parse_amount_value since it only keeps digits and '.'
        assert_eq!(parse_amount_value("€5.50"), Some(5.5));
    }

    #[test]
    fn parse_amount_value_empty() {
        assert_eq!(parse_amount_value(""), None);
    }

    #[test]
    fn parse_amount_value_no_digits() {
        assert_eq!(parse_amount_value("$"), None);
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
                    is_moderator: false,
                    is_member: false,
                    is_verified: false,
                    badges: vec![],
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
                    is_moderator: false,
                    is_member: true,
                    is_verified: false,
                    badges: vec!["member".to_string()],
                },
            ],
            statistics: SessionStatistics {
                total_messages: 2,
                unique_viewers: 2,
                super_chat_count: 1,
                super_chat_by_tier: SuperChatTierStats::default(),
                membership_count: 0,
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

    fn make_export_message(id: &str, author_id: &str, message_type: &str, tier: Option<SuperChatTier>) -> ExportMessage {
        ExportMessage {
            id: id.to_string(),
            timestamp: "2025-01-14T14:00:00Z".to_string(),
            author: "TestUser".to_string(),
            author_id: author_id.to_string(),
            content: "test content".to_string(),
            message_type: message_type.to_string(),
            amount_display: None,
            tier,
            is_moderator: false,
            is_member: false,
            is_verified: false,
            badges: vec![],
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

        let stats = calculate_session_statistics(&messages);

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

        let stats = calculate_session_statistics(&messages);

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

        let stats = calculate_session_statistics(&messages);

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

        let stats = calculate_session_statistics(&messages);

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

        let stats = calculate_session_statistics(&messages);

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
                "UC_a", "UserA",
                MessageType::SuperChat { amount: "$10.00".to_string() },
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
                "UC_b", "UserB",
                MessageType::SuperChat { amount: "$200.00".to_string() },
                Some(MessageMetadata {
                    superchat_colors: Some(SuperChatColors {
                        header_background: "#e62117".to_string(), // Red tier
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
                "UC_c", "UserC",
                MessageType::SuperSticker { amount: "$5.00".to_string() },
                None,
            ),
            make_chat_message(
                "UC_d", "UserD",
                MessageType::Membership { milestone_months: Some(6) },
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
                    MessageType::SuperChat { amount: "$5.00".to_string() },
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
                "UC_a", "UserA",
                MessageType::SuperChat { amount: "$200.00".to_string() },
                Some(MessageMetadata {
                    superchat_colors: Some(SuperChatColors {
                        header_background: "#e62117".to_string(),
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
                "UC_a", "UserA",
                MessageType::SuperChat { amount: "$10.00".to_string() },
                None,
            ),
            // UC_b: SC×2, 最高tier=Blue
            make_chat_message(
                "UC_b", "UserB",
                MessageType::SuperChat { amount: "$1.00".to_string() },
                None,
            ),
            make_chat_message(
                "UC_b", "UserB",
                MessageType::SuperChat { amount: "$1.00".to_string() },
                None,
            ),
            // UC_c: SC×1, 最高tier=Yellow
            make_chat_message(
                "UC_c", "UserC",
                MessageType::SuperChat { amount: "$15.00".to_string() },
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
        let messages = vec![
            make_chat_message(
                "UC_a", "UserA",
                MessageType::MembershipGift { gift_count: 5 },
                None,
            ),
        ];

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
                "UC_x", "UserX",
                MessageType::SuperChat { amount: "$1.00".to_string() },
                None,
            ),
            // 2件目: Red tier
            make_chat_message(
                "UC_x", "UserX",
                MessageType::SuperChat { amount: "$200.00".to_string() },
                Some(MessageMetadata {
                    superchat_colors: Some(SuperChatColors {
                        header_background: "#e62117".to_string(),
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
        assert_eq!(analytics.top_contributors[0].highest_tier, Some(SuperChatTier::Red));
    }

    #[test]
    fn compute_revenue_analytics_supersticker_contributor_count() {
        // SuperStickerもcontributor件数にカウントされること
        let messages = vec![
            make_chat_message(
                "UC_s", "StickerUser",
                MessageType::SuperSticker { amount: "$5.00".to_string() },
                None,
            ),
        ];

        let analytics = compute_revenue_analytics(&messages);
        assert_eq!(analytics.top_contributors.len(), 1);
        assert_eq!(analytics.top_contributors[0].super_chat_count, 1);
    }

    // ========================================================================
    // compute_session_analytics_from_rows (07_revenue.md: DB行データから集計)
    // ========================================================================

    #[test]
    fn compute_session_analytics_from_rows_mixed() {
        // 07_revenue.md: superchat行 + supersticker行 + membership行 → 正しい集計
        let rows = vec![
            ("superchat".to_string(), Some("$10.00".to_string()), Some("#ffb300".to_string())), // Yellow
            ("superchat".to_string(), Some("$200.00".to_string()), Some("#e62117".to_string())), // Red
            ("supersticker".to_string(), Some("$5.00".to_string()), None),
            ("membership".to_string(), None, None),
            ("membership_gift".to_string(), None, None),
            ("text".to_string(), None, None),
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
    fn compute_session_analytics_from_rows_empty() {
        // 07_revenue.md: 空の行リスト → デフォルトのRevenueAnalytics
        let analytics = compute_session_analytics_from_rows(&[]);

        assert_eq!(analytics.super_chat_count, 0);
        assert_eq!(analytics.super_sticker_count, 0);
        assert_eq!(analytics.membership_gains, 0);
        assert_eq!(analytics.super_chat_by_tier.total(), 0);
    }

    #[test]
    fn compute_session_analytics_from_rows_superchat_fallback_to_amount() {
        // 07_revenue.md: header_colorがNoneの場合、amountからtierをフォールバック判定
        let rows = vec![
            ("superchat".to_string(), Some("$200.00".to_string()), None), // amountフォールバック → Red
        ];

        let analytics = compute_session_analytics_from_rows(&rows);

        assert_eq!(analytics.super_chat_count, 1);
        assert_eq!(analytics.super_chat_by_tier.tier_red, 1);
    }

    #[test]
    fn compute_session_analytics_from_rows_superchat_no_color_no_amount() {
        // 07_revenue.md: header_colorもamountもNoneの場合 → Blue(デフォルト)
        let rows = vec![
            ("superchat".to_string(), None, None),
        ];

        let analytics = compute_session_analytics_from_rows(&rows);

        assert_eq!(analytics.super_chat_count, 1);
        assert_eq!(analytics.super_chat_by_tier.tier_blue, 1);
    }

    // ========================================================================
    // convert_messages_to_export (07_revenue.md: ChatMessage→ExportMessage変換)
    // ========================================================================

    #[test]
    fn convert_messages_to_export_text() {
        // 07_revenue.md: TextメッセージはExportMessageのmessage_type="text"に変換
        let messages = vec![
            ChatMessage {
                id: "msg1".to_string(),
                timestamp: "2025-01-14T14:00:00Z".to_string(),
                author: "TestUser".to_string(),
                channel_id: "UC_test".to_string(),
                content: "Hello".to_string(),
                message_type: MessageType::Text,
                is_member: false,
                ..Default::default()
            },
        ];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].message_type, "text");
        assert_eq!(exports[0].id, "msg1");
        assert_eq!(exports[0].author, "TestUser");
        assert_eq!(exports[0].author_id, "UC_test");
        assert_eq!(exports[0].content, "Hello");
        assert!(exports[0].amount_display.is_none());
        assert!(exports[0].tier.is_none());
        assert!(!exports[0].is_moderator);
        assert!(!exports[0].is_member);
        assert!(!exports[0].is_verified);
    }

    #[test]
    fn convert_messages_to_export_superchat_with_color() {
        // 07_revenue.md: SuperChatは色情報からtierを判定し、amountとtierを含む
        let messages = vec![
            ChatMessage {
                id: "sc1".to_string(),
                timestamp: "2025-01-14T14:01:00Z".to_string(),
                author: "SCUser".to_string(),
                channel_id: "UC_sc".to_string(),
                content: "Super!".to_string(),
                message_type: MessageType::SuperChat { amount: "$50.00".to_string() },
                metadata: Some(MessageMetadata {
                    superchat_colors: Some(SuperChatColors {
                        header_background: "#e91e63".to_string(), // Magenta
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
            },
        ];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].message_type, "superchat");
        assert_eq!(exports[0].amount_display, Some("$50.00".to_string()));
        assert_eq!(exports[0].tier, Some(SuperChatTier::Magenta));
        assert!(exports[0].is_moderator);
        assert!(exports[0].is_member);
        assert_eq!(exports[0].badges, vec!["member".to_string()]);
    }

    #[test]
    fn convert_messages_to_export_supersticker() {
        // 07_revenue.md: SuperStickerはtierなし、amountあり
        let messages = vec![
            ChatMessage {
                id: "ss1".to_string(),
                message_type: MessageType::SuperSticker { amount: "$5.00".to_string() },
                ..Default::default()
            },
        ];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports[0].message_type, "supersticker");
        assert_eq!(exports[0].amount_display, Some("$5.00".to_string()));
        assert!(exports[0].tier.is_none());
    }

    #[test]
    fn convert_messages_to_export_membership() {
        // 07_revenue.md: Membershipはmessage_type="membership"
        let messages = vec![
            ChatMessage {
                id: "m1".to_string(),
                message_type: MessageType::Membership { milestone_months: Some(12) },
                ..Default::default()
            },
        ];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports[0].message_type, "membership");
        assert!(exports[0].amount_display.is_none());
        assert!(exports[0].tier.is_none());
    }

    #[test]
    fn convert_messages_to_export_membership_gift() {
        // 07_revenue.md: MembershipGiftはmessage_type="membership_gift"
        let messages = vec![
            ChatMessage {
                id: "mg1".to_string(),
                message_type: MessageType::MembershipGift { gift_count: 5 },
                ..Default::default()
            },
        ];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports[0].message_type, "membership_gift");
    }

    #[test]
    fn convert_messages_to_export_system() {
        // 07_revenue.md: Systemはmessage_type="system"
        let messages = vec![
            ChatMessage {
                id: "sys1".to_string(),
                message_type: MessageType::System,
                ..Default::default()
            },
        ];

        let exports = convert_messages_to_export(&messages, "session1", "UC_broadcaster");

        assert_eq!(exports[0].message_type, "system");
        assert!(exports[0].amount_display.is_none());
        assert!(exports[0].tier.is_none());
    }

    #[test]
    fn convert_messages_to_export_all_types() {
        // 07_revenue.md: 全MessageTypeを含むリストが正しく変換される
        let messages = vec![
            ChatMessage { id: "1".to_string(), message_type: MessageType::Text, ..Default::default() },
            ChatMessage { id: "2".to_string(), message_type: MessageType::SuperChat { amount: "$10.00".to_string() }, ..Default::default() },
            ChatMessage { id: "3".to_string(), message_type: MessageType::SuperSticker { amount: "$3.00".to_string() }, ..Default::default() },
            ChatMessage { id: "4".to_string(), message_type: MessageType::Membership { milestone_months: None }, ..Default::default() },
            ChatMessage { id: "5".to_string(), message_type: MessageType::MembershipGift { gift_count: 1 }, ..Default::default() },
            ChatMessage { id: "6".to_string(), message_type: MessageType::System, ..Default::default() },
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
                "UC_s", "StickerUser",
                MessageType::SuperSticker { amount: "$5.00".to_string() },
                None,
            ),
            make_chat_message(
                "UC_s", "StickerUser",
                MessageType::SuperSticker { amount: "$5.00".to_string() },
                None,
            ),
            make_chat_message(
                "UC_s", "StickerUser",
                MessageType::SuperSticker { amount: "$5.00".to_string() },
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
}
