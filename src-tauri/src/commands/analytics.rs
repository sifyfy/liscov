//! Analytics and export commands
//!
//! Implements 07_revenue.md specification
//! Note: SuperChat amounts are NOT calculated numerically due to different currencies.
//! Instead, we use tier-based aggregation based on YouTube's color scheme.

use crate::core::MessageType;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tauri::State;

/// SuperChat tier based on YouTube color scheme
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorInfo {
    pub channel_id: String,
    pub display_name: String,
    pub super_chat_count: usize,
    pub highest_tier: Option<SuperChatTier>,
}

/// Hourly statistics (07_revenue.md)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyStats {
    pub hour: String,
    pub super_chat_count: usize,
    pub super_sticker_count: usize,
    pub membership_count: usize,
    pub message_count: usize,
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        // Red tier (highest)
        c if c.contains("e62117") || c.contains("ff0000") || c.starts_with("e6") => SuperChatTier::Red,
        // Magenta tier
        c if c.contains("e91e63") || c.contains("c2185b") => SuperChatTier::Magenta,
        // Orange tier
        c if c.contains("ff5722") || c.contains("e65100") || c.contains("f57c00") => SuperChatTier::Orange,
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

/// Get revenue analytics for current session
#[tauri::command]
pub async fn get_revenue_analytics(
    state: State<'_, AppState>,
) -> Result<RevenueAnalytics, String> {
    let messages = state.messages.read().await;
    let mut analytics = RevenueAnalytics::default();

    // Track contributors: channel_id -> (display_name, count, highest_tier)
    let mut contributors: HashMap<String, (String, usize, Option<SuperChatTier>)> = HashMap::new();

    for message in messages.iter() {
        match &message.message_type {
            MessageType::SuperChat { amount } => {
                analytics.super_chat_count += 1;

                // Determine tier from color if available, otherwise from amount
                let tier = if let Some(ref metadata) = message.metadata {
                    if let Some(ref colors) = metadata.superchat_colors {
                        determine_tier_from_color(&colors.header_color)
                    } else {
                        determine_tier_from_amount(amount)
                    }
                } else {
                    determine_tier_from_amount(amount)
                };

                analytics.super_chat_by_tier.increment(tier);

                // Track contributor
                let entry = contributors
                    .entry(message.channel_id.clone())
                    .or_insert((message.author.clone(), 0, None));
                entry.1 += 1;
                // Update highest tier if this one is higher
                if entry.2.is_none() || tier > entry.2.unwrap() {
                    entry.2 = Some(tier);
                }
            }
            MessageType::SuperSticker { amount: _ } => {
                analytics.super_sticker_count += 1;

                // SuperSticker counts but doesn't affect tier stats
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

    // Build top contributors list sorted by count, then by highest tier
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

    // Sort by count descending, then by tier descending
    contributors_vec.sort_by(|a, b| {
        match b.super_chat_count.cmp(&a.super_chat_count) {
            std::cmp::Ordering::Equal => b.highest_tier.cmp(&a.highest_tier),
            other => other,
        }
    });

    // Keep top 10
    contributors_vec.truncate(10);
    analytics.top_contributors = contributors_vec;

    Ok(analytics)
}

/// Get analytics for a specific session from database
#[tauri::command]
pub async fn get_session_analytics(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<RevenueAnalytics, String> {
    let db_guard = state.database.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let conn = db.connection().await;

    // Get messages for session with color info
    let mut stmt = conn
        .prepare(
            "SELECT message_type, amount, header_color FROM messages WHERE session_id = ?"
        )
        .map_err(|e| e.to_string())?;

    let mut analytics = RevenueAnalytics::default();

    let rows = stmt
        .query_map([&session_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (message_type, amount_str, header_color) = row.map_err(|e| e.to_string())?;

        match message_type.as_str() {
            "superchat" => {
                analytics.super_chat_count += 1;

                let tier = if let Some(ref color) = header_color {
                    determine_tier_from_color(color)
                } else if let Some(ref amt) = amount_str {
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

    Ok(analytics)
}

/// Export session data to file
#[tauri::command]
pub async fn export_session_data(
    state: State<'_, AppState>,
    session_id: String,
    file_path: String,
    config: ExportConfig,
) -> Result<(), String> {
    let db_guard = state.database.read().await;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let conn = db.connection().await;

    // Get session metadata
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
        .map_err(|e| format!("Session not found: {}", e))?;

    // Get messages
    let limit_clause = config.max_records.map(|n| format!(" LIMIT {}", n)).unwrap_or_default();
    let query = format!(
        "SELECT id, timestamp, author, channel_id, content, message_type, amount, is_member,
                is_moderator, is_verified, badges, header_color
         FROM messages WHERE session_id = ? ORDER BY timestamp{}",
        limit_clause
    );

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

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
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Calculate statistics
    let mut super_chat_count = 0;
    let mut super_chat_by_tier = SuperChatTierStats::default();
    let mut membership_count = 0;
    let mut unique_viewers: std::collections::HashSet<String> = std::collections::HashSet::new();

    for msg in &messages {
        unique_viewers.insert(msg.author_id.clone());

        match msg.message_type.as_str() {
            "superchat" => {
                super_chat_count += 1;
                if let Some(tier) = msg.tier {
                    super_chat_by_tier.increment(tier);
                }
            }
            "supersticker" => {
                // Counted but not in tier stats
            }
            "membership" | "membership_gift" => {
                membership_count += 1;
            }
            _ => {}
        }
    }

    let total_messages = messages.len();

    let export_data = SessionExportData {
        metadata: session,
        messages,
        statistics: SessionStatistics {
            total_messages,
            unique_viewers: unique_viewers.len(),
            super_chat_count,
            super_chat_by_tier,
            membership_count,
        },
    };

    // Export based on format
    let content = match config.format.as_str() {
        "json" => export_to_json(&export_data, &config)?,
        "csv" => export_to_csv(&export_data, &config)?,
        _ => return Err(format!("Unsupported format: {}", config.format)),
    };

    // Write to file
    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

/// Export current session messages
#[tauri::command]
pub async fn export_current_messages(
    state: State<'_, AppState>,
    file_path: String,
    config: ExportConfig,
) -> Result<(), String> {
    let messages = state.messages.read().await;
    let session_id = state.current_session_id.read().await.clone();
    let broadcaster_id = state.current_broadcaster_id.read().await.clone();

    let export_messages: Vec<ExportMessage> = messages
        .iter()
        .take(config.max_records.unwrap_or(usize::MAX))
        .map(|msg| {
            let (message_type_str, amount_display, tier) = match &msg.message_type {
                MessageType::Text => ("text".to_string(), None, None),
                MessageType::SuperChat { amount } => {
                    let t = if let Some(ref metadata) = msg.metadata {
                        if let Some(ref colors) = metadata.superchat_colors {
                            determine_tier_from_color(&colors.header_color)
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
        .collect();

    let mut unique_viewers: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut super_chat_count = 0;
    let mut super_chat_by_tier = SuperChatTierStats::default();
    let mut membership_count = 0;

    for msg in &export_messages {
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

    let export_data = SessionExportData {
        metadata: SessionMetadata {
            session_id: session_id.unwrap_or_else(|| "current".to_string()),
            stream_title: None,
            stream_url: None,
            broadcaster_name: None,
            broadcaster_channel_id: broadcaster_id,
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            export_time: Utc::now().to_rfc3339(),
        },
        statistics: SessionStatistics {
            total_messages: export_messages.len(),
            unique_viewers: unique_viewers.len(),
            super_chat_count,
            super_chat_by_tier,
            membership_count,
        },
        messages: export_messages,
    };

    let content = match config.format.as_str() {
        "json" => export_to_json(&export_data, &config)?,
        "csv" => export_to_csv(&export_data, &config)?,
        _ => return Err(format!("Unsupported format: {}", config.format)),
    };

    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

// Helper functions

fn export_to_json(data: &SessionExportData, config: &ExportConfig) -> Result<String, String> {
    if config.include_metadata {
        serde_json::to_string_pretty(data)
            .map_err(|e| format!("JSON serialization error: {}", e))
    } else {
        serde_json::to_string_pretty(&data.messages)
            .map_err(|e| format!("JSON serialization error: {}", e))
    }
}

fn export_to_csv(data: &SessionExportData, config: &ExportConfig) -> Result<String, String> {
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
