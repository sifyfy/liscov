//! CRUD operations for the database

use super::models::*;
use crate::core::models::ChatMessage;
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

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
    // Debug: Log session creation details
    tracing::info!(
        "Creating session: stream_url={:?}, stream_title={:?}, broadcaster_channel_id={:?}, broadcaster_name={:?}",
        stream_url, stream_title, broadcaster_channel_id, broadcaster_name
    );

    let id = uuid::Uuid::new_v4().to_string();
    let start_time = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO sessions (id, start_time, stream_url, stream_title, broadcaster_channel_id, broadcaster_name)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, start_time, stream_url, stream_title, broadcaster_channel_id, broadcaster_name],
    )?;

    // Also save broadcaster profile if we have broadcaster info
    if let Some(channel_id) = broadcaster_channel_id {
        let profile = BroadcasterProfile {
            channel_id: channel_id.to_string(),
            channel_name: broadcaster_name.map(|s| s.to_string()),
            handle: None,
            thumbnail_url: None,
            created_at: None,
            updated_at: None,
        };
        upsert_broadcaster_profile(conn, &profile)?;
    }

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
pub fn save_message(
    conn: &Connection,
    session_id: &str,
    broadcaster_channel_id: Option<&str>,
    message: &ChatMessage,
) -> Result<i64> {
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

    // Update viewer profile (if broadcaster_channel_id is available)
    if let Some(broadcaster_id) = broadcaster_channel_id {
        upsert_viewer_profile(
            conn,
            broadcaster_id,
            &message.channel_id,
            &message.author,
            amount.as_deref(),
        )?;
    }

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

/// Check if a viewer profile exists for a given broadcaster + channel combo
pub fn viewer_exists(conn: &Connection, broadcaster_channel_id: &str, channel_id: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM viewer_profiles WHERE broadcaster_channel_id = ?1 AND channel_id = ?2)",
        params![broadcaster_channel_id, channel_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Get in-stream comment counts per channel_id for a given video_id
pub fn get_in_stream_comment_counts(conn: &Connection, video_id: &str) -> Result<std::collections::HashMap<String, u32>> {
    let like_pattern = format!("%watch?v={}%", video_id);
    let mut stmt = conn.prepare(
        "SELECT m.channel_id, COUNT(*) as cnt
         FROM messages m
         JOIN sessions s ON m.session_id = s.id
         WHERE s.stream_url LIKE ?1
           AND m.message_type != 'system'
         GROUP BY m.channel_id",
    )?;
    let counts = stmt
        .query_map(params![like_pattern], |row| {
            let channel_id: String = row.get(0)?;
            let count: u32 = row.get(1)?;
            Ok((channel_id, count))
        })?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()?;
    Ok(counts)
}

/// Upsert viewer profile (returns the profile id)
pub fn upsert_viewer_profile(
    conn: &Connection,
    broadcaster_channel_id: &str,
    channel_id: &str,
    display_name: &str,
    amount: Option<&str>,
) -> Result<i64> {
    let now = chrono::Utc::now().to_rfc3339();
    let contribution = parse_amount(amount).unwrap_or(0.0);

    conn.execute(
        "INSERT INTO viewer_profiles (broadcaster_channel_id, channel_id, display_name, first_seen, last_seen, message_count, total_contribution)
         VALUES (?1, ?2, ?3, ?4, ?4, 1, ?5)
         ON CONFLICT(broadcaster_channel_id, channel_id) DO UPDATE SET
            display_name = excluded.display_name,
            last_seen = excluded.last_seen,
            message_count = message_count + 1,
            total_contribution = total_contribution + excluded.total_contribution",
        params![broadcaster_channel_id, channel_id, display_name, now, contribution],
    )?;

    // Get the id of the upserted row
    let id: i64 = conn.query_row(
        "SELECT id FROM viewer_profiles WHERE broadcaster_channel_id = ?1 AND channel_id = ?2",
        params![broadcaster_channel_id, channel_id],
        |row| row.get(0),
    )?;

    Ok(id)
}

/// Build ViewerProfile from a row with standard column order
fn row_to_viewer_profile(row: &rusqlite::Row) -> rusqlite::Result<ViewerProfile> {
    let tags_str: Option<String> = row.get(9)?;
    let tags = tags_str
        .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
        .unwrap_or_default();

    Ok(ViewerProfile {
        id: row.get(0)?,
        broadcaster_channel_id: row.get(1)?,
        channel_id: row.get(2)?,
        display_name: row.get(3)?,
        first_seen: row.get(4)?,
        last_seen: row.get(5)?,
        message_count: row.get(6)?,
        total_contribution: row.get(7)?,
        membership_level: row.get(8)?,
        tags,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

const VIEWER_PROFILE_COLUMNS: &str =
    "id, broadcaster_channel_id, channel_id, display_name, first_seen, last_seen, \
     message_count, total_contribution, membership_level, tags, created_at, updated_at";

/// Get viewer profile
pub fn get_viewer_profile(
    conn: &Connection,
    broadcaster_channel_id: &str,
    channel_id: &str,
) -> Result<Option<ViewerProfile>> {
    let sql = format!(
        "SELECT {} FROM viewer_profiles WHERE broadcaster_channel_id = ?1 AND channel_id = ?2",
        VIEWER_PROFILE_COLUMNS
    );
    let profile = conn
        .query_row(&sql, params![broadcaster_channel_id, channel_id], row_to_viewer_profile)
        .optional()?;
    Ok(profile)
}

/// Get viewer profile by id
pub fn get_viewer_profile_by_id(conn: &Connection, id: i64) -> Result<Option<ViewerProfile>> {
    let sql = format!(
        "SELECT {} FROM viewer_profiles WHERE id = ?1",
        VIEWER_PROFILE_COLUMNS
    );
    let profile = conn
        .query_row(&sql, params![id], row_to_viewer_profile)
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

/// Get viewer custom info by viewer_profile_id
pub fn get_viewer_custom_info(
    conn: &Connection,
    viewer_profile_id: i64,
) -> Result<Option<ViewerCustomInfo>> {
    let info = conn
        .query_row(
            "SELECT viewer_profile_id, reading, notes, custom_data, created_at, updated_at
             FROM viewer_custom_info
             WHERE viewer_profile_id = ?1",
            params![viewer_profile_id],
            |row| {
                Ok(ViewerCustomInfo {
                    viewer_profile_id: row.get(0)?,
                    reading: row.get(1)?,
                    notes: row.get(2)?,
                    custom_data: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()?;

    Ok(info)
}

/// Upsert viewer custom info
pub fn upsert_viewer_custom_info(conn: &Connection, info: &ViewerCustomInfo) -> Result<()> {
    conn.execute(
        "INSERT INTO viewer_custom_info (viewer_profile_id, reading, notes, custom_data)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(viewer_profile_id) DO UPDATE SET
            reading = excluded.reading,
            notes = excluded.notes,
            custom_data = excluded.custom_data",
        params![
            info.viewer_profile_id,
            info.reading,
            info.notes,
            info.custom_data,
        ],
    )?;

    Ok(())
}

/// Delete viewer custom info by viewer_profile_id
pub fn delete_viewer_custom_info(conn: &Connection, viewer_profile_id: i64) -> Result<bool> {
    let deleted = conn.execute(
        "DELETE FROM viewer_custom_info WHERE viewer_profile_id = ?1",
        params![viewer_profile_id],
    )?;

    Ok(deleted > 0)
}

/// Update viewer profile tags by id
pub fn update_viewer_tags(
    conn: &Connection,
    viewer_profile_id: i64,
    tags: Option<Vec<String>>,
) -> Result<bool> {
    let tags_str = tags.map(|t| t.join(","));
    let updated = conn.execute(
        "UPDATE viewer_profiles SET tags = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![tags_str, viewer_profile_id],
    )?;

    Ok(updated > 0)
}

/// Delete broadcaster and all associated viewer profiles (cascade deletes viewer_custom_info)
/// Returns (broadcaster_deleted, viewers_deleted_count)
pub fn delete_broadcaster(
    conn: &Connection,
    broadcaster_channel_id: &str,
) -> Result<(bool, u32)> {
    // Delete all viewer profiles for this broadcaster
    // (viewer_custom_info is cascade deleted via FK)
    let viewers_deleted = conn.execute(
        "DELETE FROM viewer_profiles WHERE broadcaster_channel_id = ?1",
        params![broadcaster_channel_id],
    )? as u32;

    // Delete the broadcaster profile
    let broadcaster_deleted = conn.execute(
        "DELETE FROM broadcaster_profiles WHERE channel_id = ?1",
        params![broadcaster_channel_id],
    )? > 0;

    Ok((broadcaster_deleted, viewers_deleted))
}

/// Delete viewer profile by id (cascade deletes viewer_custom_info)
pub fn delete_viewer_profile(conn: &Connection, viewer_profile_id: i64) -> Result<bool> {
    let deleted = conn.execute(
        "DELETE FROM viewer_profiles WHERE id = ?1",
        params![viewer_profile_id],
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
        "SELECT vp.id, vp.broadcaster_channel_id, vp.channel_id, vp.display_name,
                vp.first_seen, vp.last_seen, vp.message_count, vp.total_contribution,
                vp.membership_level, vp.tags,
                vci.reading, vci.notes, vci.custom_data
         FROM viewer_profiles vp
         LEFT JOIN viewer_custom_info vci ON vp.id = vci.viewer_profile_id
         WHERE vp.broadcaster_channel_id = ?1
           AND (vp.display_name LIKE ?2 OR vci.reading LIKE ?2 OR vci.notes LIKE ?2)
         ORDER BY vp.last_seen DESC
         LIMIT ?3 OFFSET ?4"
    } else {
        "SELECT vp.id, vp.broadcaster_channel_id, vp.channel_id, vp.display_name,
                vp.first_seen, vp.last_seen, vp.message_count, vp.total_contribution,
                vp.membership_level, vp.tags,
                vci.reading, vci.notes, vci.custom_data
         FROM viewer_profiles vp
         LEFT JOIN viewer_custom_info vci ON vp.id = vci.viewer_profile_id
         WHERE vp.broadcaster_channel_id = ?1
         ORDER BY vp.last_seen DESC
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
    let tags_str: Option<String> = row.get(9)?;
    let tags = tags_str
        .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
        .unwrap_or_default();

    Ok(ViewerWithCustomInfo {
        id: row.get(0)?,
        broadcaster_channel_id: row.get(1)?,
        channel_id: row.get(2)?,
        display_name: row.get(3)?,
        first_seen: row.get(4)?,
        last_seen: row.get(5)?,
        message_count: row.get(6)?,
        total_contribution: row.get(7)?,
        membership_level: row.get(8)?,
        tags,
        reading: row.get(10)?,
        notes: row.get(11)?,
        custom_data: row.get(12)?,
    })
}

/// Get viewer count for a broadcaster
pub fn get_viewer_count_for_broadcaster(conn: &Connection, broadcaster_channel_id: &str) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM viewer_profiles WHERE broadcaster_channel_id = ?1",
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
    use crate::core::models::{ChatMessage, MessageType};
    use crate::database::Database;

    fn setup_db() -> Database {
        Database::new_in_memory().expect("Failed to create in-memory database")
    }

    fn make_text_message(id: &str, author: &str, channel_id: &str, content: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            timestamp: "12:00:00".to_string(),
            timestamp_usec: "1000000".to_string(),
            message_type: MessageType::Text,
            author: author.to_string(),
            author_icon_url: None,
            channel_id: channel_id.to_string(),
            content: content.to_string(),
            runs: vec![],
            metadata: None,
            is_member: false,
            is_first_time_viewer: false,
            in_stream_comment_count: None,
        }
    }

    fn make_superchat_message(id: &str, author: &str, channel_id: &str, amount: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            timestamp: "12:00:00".to_string(),
            timestamp_usec: "1000000".to_string(),
            message_type: MessageType::SuperChat { amount: amount.to_string() },
            author: author.to_string(),
            author_icon_url: None,
            channel_id: channel_id.to_string(),
            content: "スパチャ".to_string(),
            runs: vec![],
            metadata: None,
            is_member: false,
            is_first_time_viewer: false,
            in_stream_comment_count: None,
        }
    }

    // ========================================================================
    // parse_amount (08_database.md: 金額パース)
    // ========================================================================

    #[test]
    fn test_parse_amount() {
        assert_eq!(parse_amount(Some("$10.00")), Some(10.0));
        assert_eq!(parse_amount(Some("¥1,000")), Some(1000.0));
        assert_eq!(parse_amount(Some("€5,50")), Some(5.5));
        assert_eq!(parse_amount(Some("R$ 1.234,56")), Some(1234.56));
        assert_eq!(parse_amount(None), None);
    }

    // ========================================================================
    // Session CRUD (08_database.md: セッション管理)
    // ========================================================================

    #[tokio::test]
    async fn session_create_returns_uuid() {
        let db = setup_db();
        let conn = db.connection().await;
        let id = create_session(&conn, Some("https://youtube.com/watch?v=test"), Some("Test Stream"), Some("UC_broadcaster"), Some("Broadcaster")).unwrap();

        // UUID v4 format: 8-4-4-4-12
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().filter(|c| *c == '-').count(), 4);
    }

    #[tokio::test]
    async fn session_create_sets_start_time() {
        let db = setup_db();
        let conn = db.connection().await;
        let id = create_session(&conn, Some("https://youtube.com"), Some("Title"), None, None).unwrap();

        let session = get_session(&conn, &id).unwrap().unwrap();
        assert!(!session.start_time.is_empty());
        assert!(session.end_time.is_none());
    }

    #[tokio::test]
    async fn session_end_sets_end_time() {
        let db = setup_db();
        let conn = db.connection().await;
        let id = create_session(&conn, None, None, None, None).unwrap();

        end_session(&conn, &id).unwrap();

        let session = get_session(&conn, &id).unwrap().unwrap();
        assert!(session.end_time.is_some());
    }

    #[tokio::test]
    async fn sessions_list_sorted_desc_with_limit() {
        let db = setup_db();
        let conn = db.connection().await;

        let _id1 = create_session(&conn, None, Some("First"), None, None).unwrap();
        let _id2 = create_session(&conn, None, Some("Second"), None, None).unwrap();
        let id3 = create_session(&conn, None, Some("Third"), None, None).unwrap();

        // limit=2 should return only 2 most recent
        let sessions = get_sessions(&conn, 2).unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, id3); // most recent first
    }

    // ========================================================================
    // Message Operations (08_database.md: メッセージ保存)
    // ========================================================================

    #[tokio::test]
    async fn message_save_and_retrieve() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, Some("UC_bc"), Some("BC")).unwrap();

        let msg = make_text_message("msg1", "User1", "UC_user1", "Hello");
        save_message(&conn, &session_id, Some("UC_bc"), &msg).unwrap();

        let messages = get_session_messages(&conn, &session_id, 100).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_id, "msg1");
        assert_eq!(messages[0].author, "User1");
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[0].message_type, "text");
    }

    #[tokio::test]
    async fn message_deduplication() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, None, None).unwrap();

        let msg = make_text_message("dup_msg", "User", "UC_user", "Content");
        save_message(&conn, &session_id, None, &msg).unwrap();
        // INSERT OR IGNORE should not fail on duplicate
        save_message(&conn, &session_id, None, &msg).unwrap();

        let messages = get_session_messages(&conn, &session_id, 100).unwrap();
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn messages_filtered_by_session() {
        let db = setup_db();
        let conn = db.connection().await;
        let session1 = create_session(&conn, None, None, None, None).unwrap();
        let session2 = create_session(&conn, None, None, None, None).unwrap();

        save_message(&conn, &session1, None, &make_text_message("m1", "A", "UC_a", "msg1")).unwrap();
        save_message(&conn, &session2, None, &make_text_message("m2", "B", "UC_b", "msg2")).unwrap();

        let msgs1 = get_session_messages(&conn, &session1, 100).unwrap();
        let msgs2 = get_session_messages(&conn, &session2, 100).unwrap();

        assert_eq!(msgs1.len(), 1);
        assert_eq!(msgs1[0].message_id, "m1");
        assert_eq!(msgs2.len(), 1);
        assert_eq!(msgs2[0].message_id, "m2");
    }

    #[tokio::test]
    async fn messages_limit() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, None, None).unwrap();

        for i in 0..5 {
            let msg = make_text_message(&format!("m{}", i), "User", "UC_u", &format!("msg{}", i));
            save_message(&conn, &session_id, None, &msg).unwrap();
        }

        let messages = get_session_messages(&conn, &session_id, 3).unwrap();
        assert_eq!(messages.len(), 3);
    }

    // ========================================================================
    // Viewer Profile (06_viewer.md + 08_database.md: 視聴者プロフィール)
    // ========================================================================

    #[tokio::test]
    async fn viewer_profile_created_on_first_message() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, Some("UC_bc"), Some("BC")).unwrap();

        let msg = make_text_message("m1", "Viewer1", "UC_viewer1", "hi");
        save_message(&conn, &session_id, Some("UC_bc"), &msg).unwrap();

        let profile = get_viewer_profile(&conn, "UC_bc", "UC_viewer1").unwrap().unwrap();
        assert_eq!(profile.display_name, "Viewer1");
        assert_eq!(profile.message_count, 1);
        assert_eq!(profile.total_contribution, 0.0);
    }

    #[tokio::test]
    async fn viewer_profile_updated_on_subsequent_messages() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, Some("UC_bc"), Some("BC")).unwrap();

        let msg1 = make_text_message("m1", "Viewer1", "UC_v1", "first");
        save_message(&conn, &session_id, Some("UC_bc"), &msg1).unwrap();

        let msg2 = make_text_message("m2", "Viewer1", "UC_v1", "second");
        save_message(&conn, &session_id, Some("UC_bc"), &msg2).unwrap();

        let profile = get_viewer_profile(&conn, "UC_bc", "UC_v1").unwrap().unwrap();
        assert_eq!(profile.message_count, 2);
    }

    #[tokio::test]
    async fn viewer_contribution_incremented_on_superchat() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, Some("UC_bc"), Some("BC")).unwrap();

        let sc = make_superchat_message("sc1", "BigFan", "UC_fan", "$50.00");
        save_message(&conn, &session_id, Some("UC_bc"), &sc).unwrap();

        let profile = get_viewer_profile(&conn, "UC_bc", "UC_fan").unwrap().unwrap();
        assert_eq!(profile.message_count, 1);
        assert!(profile.total_contribution > 0.0);
    }

    // ========================================================================
    // Broadcaster Scoping (06_viewer.md: 配信者別スコープ)
    // ========================================================================

    #[tokio::test]
    async fn viewer_scoped_per_broadcaster() {
        let db = setup_db();
        let conn = db.connection().await;
        let s1 = create_session(&conn, None, None, Some("UC_bcA"), Some("BroadcasterA")).unwrap();
        let s2 = create_session(&conn, None, None, Some("UC_bcB"), Some("BroadcasterB")).unwrap();

        // Same viewer on different broadcasters
        save_message(&conn, &s1, Some("UC_bcA"), &make_text_message("m1", "CommonViewer", "UC_common", "hi")).unwrap();
        save_message(&conn, &s1, Some("UC_bcA"), &make_text_message("m2", "CommonViewer", "UC_common", "hello")).unwrap();
        save_message(&conn, &s2, Some("UC_bcB"), &make_text_message("m3", "CommonViewer", "UC_common", "hey")).unwrap();

        let profile_a = get_viewer_profile(&conn, "UC_bcA", "UC_common").unwrap().unwrap();
        let profile_b = get_viewer_profile(&conn, "UC_bcB", "UC_common").unwrap().unwrap();

        assert_eq!(profile_a.message_count, 2);
        assert_eq!(profile_b.message_count, 1);
    }

    // ========================================================================
    // Viewer Custom Info (06_viewer.md: カスタム情報)
    // ========================================================================

    #[tokio::test]
    async fn viewer_custom_info_upsert_and_retrieve() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, Some("UC_bc"), Some("BC")).unwrap();

        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m1", "User", "UC_u", "hi")).unwrap();
        let profile = get_viewer_profile(&conn, "UC_bc", "UC_u").unwrap().unwrap();

        let info = ViewerCustomInfo::new(profile.id)
            .with_reading("やまだ たろう")
            .with_notes("常連さん");
        upsert_viewer_custom_info(&conn, &info).unwrap();

        let loaded = get_viewer_custom_info(&conn, profile.id).unwrap().unwrap();
        assert_eq!(loaded.reading.as_deref(), Some("やまだ たろう"));
        assert_eq!(loaded.notes.as_deref(), Some("常連さん"));
    }

    #[tokio::test]
    async fn viewer_custom_info_cascade_delete() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, Some("UC_bc"), Some("BC")).unwrap();

        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m1", "User", "UC_u", "hi")).unwrap();
        let profile = get_viewer_profile(&conn, "UC_bc", "UC_u").unwrap().unwrap();

        let info = ViewerCustomInfo::new(profile.id).with_reading("test");
        upsert_viewer_custom_info(&conn, &info).unwrap();

        // Delete viewer profile → custom info should be cascade deleted
        delete_viewer_profile(&conn, profile.id).unwrap();

        let loaded = get_viewer_custom_info(&conn, profile.id).unwrap();
        assert!(loaded.is_none());
    }

    // ========================================================================
    // Broadcaster Operations (06_viewer.md: 配信者管理)
    // ========================================================================

    #[tokio::test]
    async fn broadcaster_profile_created_with_session() {
        let db = setup_db();
        let conn = db.connection().await;
        create_session(&conn, None, None, Some("UC_test_bc"), Some("TestBroadcaster")).unwrap();

        let profile = get_broadcaster_profile(&conn, "UC_test_bc").unwrap().unwrap();
        assert_eq!(profile.channel_name.as_deref(), Some("TestBroadcaster"));
    }

    #[tokio::test]
    async fn delete_broadcaster_cascades() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, Some("UC_bc"), Some("BC")).unwrap();

        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m1", "V1", "UC_v1", "hi")).unwrap();
        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m2", "V2", "UC_v2", "hello")).unwrap();

        let (deleted, viewer_count) = delete_broadcaster(&conn, "UC_bc").unwrap();
        assert!(deleted);
        assert_eq!(viewer_count, 2);

        // Viewers should be gone
        let profile = get_viewer_profile(&conn, "UC_bc", "UC_v1").unwrap();
        assert!(profile.is_none());
    }

    // ========================================================================
    // Session Stats Update (08_database.md: 統計更新)
    // ========================================================================

    #[tokio::test]
    async fn session_stats_updated() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, None, None).unwrap();

        save_message(&conn, &session_id, None, &make_text_message("m1", "U", "UC_u", "hi")).unwrap();
        save_message(&conn, &session_id, None, &make_superchat_message("sc1", "U", "UC_u", "$10.00")).unwrap();

        update_session_stats(&conn, &session_id).unwrap();

        let session = get_session(&conn, &session_id).unwrap().unwrap();
        assert_eq!(session.total_messages, 2);
        assert!(session.total_revenue > 0.0);
    }

    // ========================================================================
    // viewer_exists (first-time viewer detection)
    // ========================================================================

    #[tokio::test]
    async fn viewer_exists_returns_false_for_unknown_viewer() {
        let db = setup_db();
        let conn = db.connection().await;
        let exists = viewer_exists(&conn, "UC_bc", "UC_unknown").unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn viewer_exists_returns_true_after_upsert() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, Some("UC_bc"), Some("BC")).unwrap();

        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m1", "Viewer", "UC_viewer", "hi")).unwrap();

        let exists = viewer_exists(&conn, "UC_bc", "UC_viewer").unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn viewer_exists_is_scoped_per_broadcaster() {
        let db = setup_db();
        let conn = db.connection().await;
        let session_id = create_session(&conn, None, None, Some("UC_bcA"), Some("BcA")).unwrap();

        save_message(&conn, &session_id, Some("UC_bcA"), &make_text_message("m1", "Viewer", "UC_viewer", "hi")).unwrap();

        // Exists for bcA but not bcB
        assert!(viewer_exists(&conn, "UC_bcA", "UC_viewer").unwrap());
        assert!(!viewer_exists(&conn, "UC_bcB", "UC_viewer").unwrap());
    }

    // ========================================================================
    // get_in_stream_comment_counts (in-stream comment count aggregation)
    // ========================================================================

    #[tokio::test]
    async fn get_in_stream_comment_counts_returns_empty_for_unknown_video() {
        let db = setup_db();
        let conn = db.connection().await;
        let counts = get_in_stream_comment_counts(&conn, "nonexistent_video").unwrap();
        assert!(counts.is_empty());
    }

    #[tokio::test]
    async fn get_in_stream_comment_counts_returns_message_counts_per_channel() {
        let db = setup_db();
        let conn = db.connection().await;
        let video_id = "dQw4w9WgXcQ";
        let stream_url = format!("https://www.youtube.com/watch?v={}", video_id);
        let session_id = create_session(&conn, Some(&stream_url), Some("Test Stream"), Some("UC_bc"), Some("BC")).unwrap();

        // User A sends 3 messages, User B sends 2
        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m1", "A", "UC_a", "hi1")).unwrap();
        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m2", "A", "UC_a", "hi2")).unwrap();
        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m3", "A", "UC_a", "hi3")).unwrap();
        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m4", "B", "UC_b", "hey1")).unwrap();
        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m5", "B", "UC_b", "hey2")).unwrap();

        let counts = get_in_stream_comment_counts(&conn, video_id).unwrap();
        assert_eq!(counts.get("UC_a"), Some(&3u32));
        assert_eq!(counts.get("UC_b"), Some(&2u32));
    }

    #[tokio::test]
    async fn get_in_stream_comment_counts_does_not_count_system_messages() {
        let db = setup_db();
        let conn = db.connection().await;
        let video_id = "testVideo123";
        let stream_url = format!("https://www.youtube.com/watch?v={}", video_id);
        let session_id = create_session(&conn, Some(&stream_url), None, Some("UC_bc"), Some("BC")).unwrap();

        let sys_msg = ChatMessage {
            id: "sys1".to_string(),
            timestamp: "12:00:00".to_string(),
            timestamp_usec: "1000000".to_string(),
            message_type: MessageType::System,
            author: "System".to_string(),
            author_icon_url: None,
            channel_id: "UC_sys".to_string(),
            content: "Stream started".to_string(),
            runs: vec![],
            metadata: None,
            is_member: false,
            is_first_time_viewer: false,
            in_stream_comment_count: None,
        };
        save_message(&conn, &session_id, Some("UC_bc"), &sys_msg).unwrap();
        save_message(&conn, &session_id, Some("UC_bc"), &make_text_message("m1", "A", "UC_a", "hi")).unwrap();

        let counts = get_in_stream_comment_counts(&conn, video_id).unwrap();
        // system messages are saved as message_type="system", but counted only for non-system?
        // spec says count all messages in session, system messages included in DB but here we count text
        // Per spec: in_stream_comment_count = count of messages by channel_id in stream
        // System messages have a channel_id, but per spec only user messages should count
        assert_eq!(counts.get("UC_sys"), None, "System messages should not be counted");
        assert_eq!(counts.get("UC_a"), Some(&1u32));
    }
}
