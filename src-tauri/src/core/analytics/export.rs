//! CSV / JSON に整えて書き出す（07_revenue.md「エクスポート」）

use super::*;
use crate::errors::CommandError;
use std::fs::File;
use std::io::Write;

/// 形式に合わせて整形し、ファイルに書き出す
pub(crate) fn write_export(
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

/// Calculate session statistics from export messages (DRY: used by both export functions)
pub(crate) fn calculate_session_statistics(
    messages: &[ExportMessage],
    gifts: GiftStats,
) -> SessionStatistics {
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

pub(crate) fn export_to_json(
    data: &SessionExportData,
    config: &ExportConfig,
) -> Result<String, CommandError> {
    if config.include_metadata {
        serde_json::to_string_pretty(data)
            .map_err(|e| CommandError::Internal(format!("JSON serialization error: {}", e)))
    } else {
        serde_json::to_string_pretty(&data.messages)
            .map_err(|e| CommandError::Internal(format!("JSON serialization error: {}", e)))
    }
}

pub(crate) fn export_to_csv(
    data: &SessionExportData,
    config: &ExportConfig,
) -> Result<String, CommandError> {
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
