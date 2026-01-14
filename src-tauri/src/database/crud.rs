//! CRUD operations for the database

use super::models::*;
use crate::core::models::ChatMessage;
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;

// ============================================================================
// Session Operations
// ============================================================================

/// Create a new session
pub fn create_session(
    conn: &Connection,
    stream_url: Option<&str>,
    stream_title: Option<&str>,
    broadcaster_channel_id: Option<&str>,
    broadcaster_name: Option<&str>,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let start_time = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO sessions (id, start_time, stream_url, stream_title, broadcaster_channel_id, broadcaster_name)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, start_time, stream_url, stream_title, broadcaster_channel_id, broadcaster_name],
    )?;

    Ok(id)
}

/// End a session
pub fn end_session(conn: &Connection, session_id: &str) -> Result<()> {
    let end_time = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE sessions SET end_time = ?1 WHERE id = ?2",
        params![end_time, session_id],
    )?;
    Ok(())
}

/// Update session statistics
pub fn update_session_stats(conn: &Connection, session_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET
            total_messages = (SELECT COUNT(*) FROM messages WHERE session_id = ?1),
            total_revenue = (SELECT COALESCE(SUM(
                CASE
                    WHEN amount IS NOT NULL THEN CAST(
                        REPLACE(REPLACE(REPLACE(REPLACE(amount, '$', ''), '¥', ''), '€', ''), ',', '')
                        AS REAL
                    )
                    ELSE 0
                END
            ), 0) FROM messages WHERE session_id = ?1 AND message_type IN ('superchat', 'supersticker'))
         WHERE id = ?1",
        params![session_id],
    )?;
    Ok(())
}

/// Get sessions list
pub fn get_sessions(conn: &Connection, limit: usize) -> Result<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, start_time, end_time, stream_url, stream_title, broadcaster_channel_id,
                broadcaster_name, total_messages, total_revenue, created_at, updated_at
         FROM sessions
         ORDER BY start_time DESC
         LIMIT ?1",
    )?;

    let sessions = stmt
        .query_map(params![limit], |row| {
            Ok(Session {
                id: row.get(0)?,
                start_time: row.get(1)?,
                end_time: row.get(2)?,
                stream_url: row.get(3)?,
                stream_title: row.get(4)?,
                broadcaster_channel_id: row.get(5)?,
                broadcaster_name: row.get(6)?,
                total_messages: row.get(7)?,
                total_revenue: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(sessions)
}

/// Get a single session by ID
pub fn get_session(conn: &Connection, session_id: &str) -> Result<Option<Session>> {
    let session = conn
        .query_row(
            "SELECT id, start_time, end_time, stream_url, stream_title, broadcaster_channel_id,
                    broadcaster_name, total_messages, total_revenue, created_at, updated_at
             FROM sessions WHERE id = ?1",
            params![session_id],
            |row| {
                Ok(Session {
                    id: row.get(0)?,
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                    stream_url: row.get(3)?,
                    stream_title: row.get(4)?,
                    broadcaster_channel_id: row.get(5)?,
                    broadcaster_name: row.get(6)?,
                    total_messages: row.get(7)?,
                    total_revenue: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .optional()?;

    Ok(session)
}

// ============================================================================
// Message Operations
// ============================================================================

/// Save a chat message
pub fn save_message(conn: &Connection, session_id: &str, message: &ChatMessage) -> Result<i64> {
    let message_type = match &message.message_type {
        crate::core::models::MessageType::Text => "text",
        crate::core::models::MessageType::SuperChat { .. } => "superchat",
        crate::core::models::MessageType::SuperSticker { .. } => "supersticker",
        crate::core::models::MessageType::Membership { .. } => "membership",
        crate::core::models::MessageType::MembershipGift { .. } => "membership_gift",
        crate::core::models::MessageType::System => "system",
    };

    let amount = match &message.message_type {
        crate::core::models::MessageType::SuperChat { amount } => Some(amount.clone()),
        crate::core::models::MessageType::SuperSticker { amount } => Some(amount.clone()),
        _ => None,
    };

    // Insert message (ignore duplicates)
    conn.execute(
        "INSERT OR IGNORE INTO messages
         (session_id, message_id, timestamp, timestamp_usec, author, author_icon_url,
          channel_id, content, message_type, amount, is_member)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            session_id,
            message.id,
            message.timestamp,
            message.timestamp_usec,
            message.author,
            message.author_icon_url,
            message.channel_id,
            message.content,
            message_type,
            amount,
            message.is_member,
        ],
    )?;

    // Update viewer profile
    upsert_viewer_profile(conn, &message.channel_id, &message.author, amount.as_deref())?;

    Ok(conn.last_insert_rowid())
}

/// Get messages for a session
pub fn get_session_messages(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<StoredMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, message_id, timestamp, timestamp_usec, author, author_icon_url,
                channel_id, content, message_type, amount, is_member, metadata, created_at
         FROM messages
         WHERE session_id = ?1
         ORDER BY timestamp DESC
         LIMIT ?2",
    )?;

    let messages = stmt
        .query_map(params![session_id, limit], |row| {
            Ok(StoredMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                message_id: row.get(2)?,
                timestamp: row.get(3)?,
                timestamp_usec: row.get(4)?,
                author: row.get(5)?,
                author_icon_url: row.get(6)?,
                channel_id: row.get(7)?,
                content: row.get(8)?,
                message_type: row.get(9)?,
                amount: row.get(10)?,
                is_member: row.get::<_, i64>(11)? != 0,
                metadata: row.get(12)?,
                created_at: row.get(13)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(messages)
}

// ============================================================================
// Viewer Profile Operations
// ============================================================================

/// Upsert viewer profile
pub fn upsert_viewer_profile(
    conn: &Connection,
    channel_id: &str,
    display_name: &str,
    amount: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let contribution = parse_amount(amount).unwrap_or(0.0);

    conn.execute(
        "INSERT INTO viewer_profiles (channel_id, display_name, first_seen, last_seen, message_count, total_contribution)
         VALUES (?1, ?2, ?3, ?3, 1, ?4)
         ON CONFLICT(channel_id) DO UPDATE SET
            display_name = excluded.display_name,
            last_seen = excluded.last_seen,
            message_count = message_count + 1,
            total_contribution = total_contribution + excluded.total_contribution",
        params![channel_id, display_name, now, contribution],
    )?;

    Ok(())
}

/// Get viewer profile
pub fn get_viewer_profile(conn: &Connection, channel_id: &str) -> Result<Option<ViewerProfile>> {
    let profile = conn
        .query_row(
            "SELECT channel_id, display_name, first_seen, last_seen, message_count,
                    total_contribution, membership_level, tags, created_at, updated_at
             FROM viewer_profiles WHERE channel_id = ?1",
            params![channel_id],
            |row| {
                let tags_str: Option<String> = row.get(7)?;
                let tags = tags_str
                    .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
                    .unwrap_or_default();

                Ok(ViewerProfile {
                    channel_id: row.get(0)?,
                    display_name: row.get(1)?,
                    first_seen: row.get(2)?,
                    last_seen: row.get(3)?,
                    message_count: row.get(4)?,
                    total_contribution: row.get(5)?,
                    membership_level: row.get(6)?,
                    tags,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .optional()?;

    Ok(profile)
}

/// Get top contributors for a session
pub fn get_top_contributors(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<ContributorStats>> {
    let mut stmt = conn.prepare(
        "SELECT m.channel_id, m.author, COUNT(*) as msg_count,
                COALESCE(SUM(CASE WHEN m.amount IS NOT NULL THEN
                    CAST(REPLACE(REPLACE(REPLACE(REPLACE(m.amount, '$', ''), '¥', ''), '€', ''), ',', '') AS REAL)
                ELSE 0 END), 0) as contribution
         FROM messages m
         WHERE m.session_id = ?1
         GROUP BY m.channel_id
         ORDER BY contribution DESC, msg_count DESC
         LIMIT ?2",
    )?;

    let contributors = stmt
        .query_map(params![session_id, limit], |row| {
            Ok(ContributorStats {
                channel_id: row.get(0)?,
                display_name: row.get(1)?,
                message_count: row.get(2)?,
                total_contribution: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(contributors)
}

// ============================================================================
// Viewer Custom Info Operations
// ============================================================================

/// Get viewer custom info
pub fn get_viewer_custom_info(
    conn: &Connection,
    broadcaster_channel_id: &str,
    viewer_channel_id: &str,
) -> Result<Option<ViewerCustomInfo>> {
    let info = conn
        .query_row(
            "SELECT id, broadcaster_channel_id, viewer_channel_id, reading, notes,
                    custom_data, created_at, updated_at
             FROM viewer_custom_info
             WHERE broadcaster_channel_id = ?1 AND viewer_channel_id = ?2",
            params![broadcaster_channel_id, viewer_channel_id],
            |row| {
                Ok(ViewerCustomInfo {
                    id: row.get(0)?,
                    broadcaster_channel_id: row.get(1)?,
                    viewer_channel_id: row.get(2)?,
                    reading: row.get(3)?,
                    notes: row.get(4)?,
                    custom_data: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .optional()?;

    Ok(info)
}

/// Upsert viewer custom info
pub fn upsert_viewer_custom_info(conn: &Connection, info: &ViewerCustomInfo) -> Result<i64> {
    conn.execute(
        "INSERT INTO viewer_custom_info (broadcaster_channel_id, viewer_channel_id, reading, notes, custom_data)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(broadcaster_channel_id, viewer_channel_id) DO UPDATE SET
            reading = excluded.reading,
            notes = excluded.notes,
            custom_data = excluded.custom_data",
        params![
            info.broadcaster_channel_id,
            info.viewer_channel_id,
            info.reading,
            info.notes,
            info.custom_data,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Get all viewer custom info for a broadcaster
pub fn get_all_viewer_custom_info_for_broadcaster(
    conn: &Connection,
    broadcaster_channel_id: &str,
) -> Result<HashMap<String, ViewerCustomInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, broadcaster_channel_id, viewer_channel_id, reading, notes,
                custom_data, created_at, updated_at
         FROM viewer_custom_info
         WHERE broadcaster_channel_id = ?1",
    )?;

    let mut map = HashMap::new();
    let rows = stmt.query_map(params![broadcaster_channel_id], |row| {
        Ok(ViewerCustomInfo {
            id: row.get(0)?,
            broadcaster_channel_id: row.get(1)?,
            viewer_channel_id: row.get(2)?,
            reading: row.get(3)?,
            notes: row.get(4)?,
            custom_data: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    for info in rows {
        let info = info?;
        map.insert(info.viewer_channel_id.clone(), info);
    }

    Ok(map)
}

/// Delete viewer custom info
pub fn delete_viewer_custom_info(
    conn: &Connection,
    broadcaster_channel_id: &str,
    viewer_channel_id: &str,
) -> Result<bool> {
    let deleted = conn.execute(
        "DELETE FROM viewer_custom_info WHERE broadcaster_channel_id = ?1 AND viewer_channel_id = ?2",
        params![broadcaster_channel_id, viewer_channel_id],
    )?;

    Ok(deleted > 0)
}

// ============================================================================
// Viewer Management Operations
// ============================================================================

/// Get viewers for a broadcaster with optional search and pagination
pub fn get_viewers_for_broadcaster(
    conn: &Connection,
    broadcaster_channel_id: &str,
    search_query: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<Vec<ViewerWithCustomInfo>> {
    let query = if search_query.is_some() {
        "SELECT vp.channel_id, vp.display_name, vp.first_seen, vp.last_seen,
                vp.message_count, vp.total_contribution, vp.membership_level, vp.tags,
                vci.reading, vci.notes, vci.custom_data
         FROM viewer_profiles vp
         LEFT JOIN viewer_custom_info vci ON vp.channel_id = vci.viewer_channel_id
            AND vci.broadcaster_channel_id = ?1
         WHERE vp.display_name LIKE ?2 OR vci.reading LIKE ?2 OR vci.notes LIKE ?2
         ORDER BY vp.message_count DESC
         LIMIT ?3 OFFSET ?4"
    } else {
        "SELECT vp.channel_id, vp.display_name, vp.first_seen, vp.last_seen,
                vp.message_count, vp.total_contribution, vp.membership_level, vp.tags,
                vci.reading, vci.notes, vci.custom_data
         FROM viewer_profiles vp
         LEFT JOIN viewer_custom_info vci ON vp.channel_id = vci.viewer_channel_id
            AND vci.broadcaster_channel_id = ?1
         ORDER BY vp.message_count DESC
         LIMIT ?3 OFFSET ?4"
    };

    let mut stmt = conn.prepare(query)?;

    let search_pattern = search_query.map(|q| format!("%{}%", q));

    let viewers = if let Some(pattern) = &search_pattern {
        stmt.query_map(params![broadcaster_channel_id, pattern, limit, offset], row_to_viewer)?
    } else {
        stmt.query_map(params![broadcaster_channel_id, "", limit, offset], row_to_viewer)?
    };

    viewers.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn row_to_viewer(row: &rusqlite::Row) -> rusqlite::Result<ViewerWithCustomInfo> {
    let tags_str: Option<String> = row.get(7)?;
    let tags = tags_str
        .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
        .unwrap_or_default();

    Ok(ViewerWithCustomInfo {
        channel_id: row.get(0)?,
        display_name: row.get(1)?,
        first_seen: row.get(2)?,
        last_seen: row.get(3)?,
        message_count: row.get(4)?,
        total_contribution: row.get(5)?,
        membership_level: row.get(6)?,
        tags,
        reading: row.get(8)?,
        notes: row.get(9)?,
        custom_data: row.get(10)?,
    })
}

/// Get viewer count for a broadcaster
pub fn get_viewer_count_for_broadcaster(conn: &Connection, broadcaster_channel_id: &str) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT vp.channel_id)
         FROM viewer_profiles vp
         LEFT JOIN viewer_custom_info vci ON vp.channel_id = vci.viewer_channel_id
            AND vci.broadcaster_channel_id = ?1",
        params![broadcaster_channel_id],
        |row| row.get(0),
    )?;

    Ok(count)
}

// ============================================================================
// Broadcaster Operations
// ============================================================================

/// Upsert broadcaster profile
pub fn upsert_broadcaster_profile(conn: &Connection, profile: &BroadcasterProfile) -> Result<()> {
    conn.execute(
        "INSERT INTO broadcaster_profiles (channel_id, channel_name, handle, thumbnail_url)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(channel_id) DO UPDATE SET
            channel_name = excluded.channel_name,
            handle = excluded.handle,
            thumbnail_url = excluded.thumbnail_url",
        params![
            profile.channel_id,
            profile.channel_name,
            profile.handle,
            profile.thumbnail_url,
        ],
    )?;

    Ok(())
}

/// Get broadcaster profile
pub fn get_broadcaster_profile(conn: &Connection, channel_id: &str) -> Result<Option<BroadcasterProfile>> {
    let profile = conn
        .query_row(
            "SELECT channel_id, channel_name, handle, thumbnail_url, created_at, updated_at
             FROM broadcaster_profiles WHERE channel_id = ?1",
            params![channel_id],
            |row| {
                Ok(BroadcasterProfile {
                    channel_id: row.get(0)?,
                    channel_name: row.get(1)?,
                    handle: row.get(2)?,
                    thumbnail_url: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()?;

    Ok(profile)
}

/// Get distinct broadcaster channels
pub fn get_distinct_broadcaster_channels(conn: &Connection) -> Result<Vec<BroadcasterProfile>> {
    let mut stmt = conn.prepare(
        "SELECT channel_id, channel_name, handle, thumbnail_url, created_at, updated_at
         FROM broadcaster_profiles
         ORDER BY channel_name",
    )?;

    let broadcasters = stmt
        .query_map([], |row| {
            Ok(BroadcasterProfile {
                channel_id: row.get(0)?,
                channel_name: row.get(1)?,
                handle: row.get(2)?,
                thumbnail_url: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(broadcasters)
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Parse amount string to f64
fn parse_amount(amount: Option<&str>) -> Option<f64> {
    let amount = amount?;
    let cleaned: String = amount
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();

    // Handle European decimal format (1.234,56 -> 1234.56)
    let normalized = if cleaned.contains(',') && cleaned.contains('.') {
        // Assume comma is decimal separator if it comes after the last dot
        let last_comma = cleaned.rfind(',').unwrap_or(0);
        let last_dot = cleaned.rfind('.').unwrap_or(0);
        if last_comma > last_dot {
            cleaned.replace('.', "").replace(',', ".")
        } else {
            cleaned.replace(',', "")
        }
    } else if cleaned.contains(',') {
        // Comma could be decimal or thousands separator
        let parts: Vec<&str> = cleaned.split(',').collect();
        if parts.len() == 2 && parts[1].len() <= 2 {
            // Likely decimal separator
            cleaned.replace(',', ".")
        } else {
            // Likely thousands separator
            cleaned.replace(',', "")
        }
    } else {
        cleaned
    };

    normalized.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_amount() {
        assert_eq!(parse_amount(Some("$10.00")), Some(10.0));
        assert_eq!(parse_amount(Some("¥1,000")), Some(1000.0));
        assert_eq!(parse_amount(Some("€5,50")), Some(5.5));
        assert_eq!(parse_amount(Some("R$ 1.234,56")), Some(1234.56));
        assert_eq!(parse_amount(None), None);
    }
}
