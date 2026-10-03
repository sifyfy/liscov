//! 集計結果・エクスポートの型（07_revenue.md「データモデル」）

use super::*;
use crate::core::GiftDetails;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
pub(crate) fn format_jewels(jewel_count: u32) -> String {
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
