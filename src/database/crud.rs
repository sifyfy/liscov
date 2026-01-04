use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Row};
use serde_json;
use uuid::Uuid;

use super::{LiscovDatabase, Question, Session, ViewerCustomInfo, ViewerProfile};
use std::collections::HashMap;
use crate::gui::models::GuiChatMessage;

impl LiscovDatabase {
    /// 新しいセッションを作成
    pub fn create_session(
        &mut self,
        stream_url: &str,
        stream_title: Option<&str>,
    ) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let start_time = Utc::now().to_rfc3339();

        self.connection.execute(
            "INSERT INTO sessions (id, start_time, stream_url, stream_title) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, start_time, stream_url, stream_title],
        )?;

        tracing::info!("Created new session: {}", session_id);
        Ok(session_id)
    }

    /// セッションを終了
    pub fn end_session(&mut self, session_id: &str) -> Result<()> {
        let end_time = Utc::now().to_rfc3339();

        self.connection.execute(
            "UPDATE sessions SET end_time = ?1 WHERE id = ?2",
            params![end_time, session_id],
        )?;

        tracing::info!("Ended session: {}", session_id);
        Ok(())
    }

    /// セッションの統計を更新
    pub fn update_session_stats(&mut self, session_id: &str) -> Result<()> {
        let mut stmt = self.connection.prepare(
            "SELECT COUNT(*) as message_count, 
                    COALESCE(SUM(amount), 0.0) as total_revenue 
             FROM messages 
             WHERE session_id = ?1",
        )?;

        let (message_count, total_revenue): (i64, f64) =
            stmt.query_row(params![session_id], |row| Ok((row.get(0)?, row.get(1)?)))?;

        self.connection.execute(
            "UPDATE sessions SET total_messages = ?1, total_revenue = ?2 WHERE id = ?3",
            params![message_count, total_revenue, session_id],
        )?;

        Ok(())
    }

    /// メッセージを保存
    pub fn save_message(&mut self, session_id: &str, message: &GuiChatMessage) -> Result<i64> {
        let amount = match &message.message_type {
            crate::gui::models::MessageType::SuperChat { amount }
            | crate::gui::models::MessageType::SuperSticker { amount } => {
                self.parse_amount_for_db(amount).unwrap_or(0.0)
            }
            _ => 0.0,
        };

        let metadata_json = if let Some(metadata) = &message.metadata {
            Some(serde_json::to_string(metadata)?)
        } else {
            None
        };

        let message_id = self
            .connection
            .prepare(
                "INSERT INTO messages 
             (session_id, timestamp, author, channel_id, content, message_type, amount, metadata) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?
            .insert(params![
                session_id,
                message.timestamp,
                message.author,
                message.channel_id,
                message.content,
                message.message_type.as_string(),
                amount,
                metadata_json,
            ])?;

        // 視聴者プロフィールを更新
        self.upsert_viewer_profile(&message.channel_id, &message.author, amount)?;

        Ok(message_id)
    }

    /// 視聴者プロフィールを作成または更新
    pub fn upsert_viewer_profile(
        &mut self,
        channel_id: &str,
        display_name: &str,
        contribution: f64,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        // 既存プロフィールをチェック
        let exists: bool = self
            .connection
            .prepare("SELECT 1 FROM viewer_profiles WHERE channel_id = ?1")?
            .exists(params![channel_id])?;

        if exists {
            // 更新
            self.connection.execute(
                "UPDATE viewer_profiles 
                 SET display_name = ?1, last_seen = ?2, 
                     message_count = message_count + 1,
                     total_contribution = total_contribution + ?3
                 WHERE channel_id = ?4",
                params![display_name, now, contribution, channel_id],
            )?;
        } else {
            // 新規作成
            self.connection.execute(
                "INSERT INTO viewer_profiles 
                 (channel_id, display_name, first_seen, last_seen, message_count, total_contribution) 
                 VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                params![channel_id, display_name, now, now, contribution],
            )?;
        }

        Ok(())
    }

    /// セッション一覧を取得
    pub fn get_sessions(&self, limit: Option<usize>) -> Result<Vec<Session>> {
        let sql = if let Some(limit) = limit {
            format!(
                "SELECT * FROM sessions ORDER BY start_time DESC LIMIT {}",
                limit
            )
        } else {
            "SELECT * FROM sessions ORDER BY start_time DESC".to_string()
        };

        let mut stmt = self.connection.prepare(&sql)?;
        let session_iter = stmt.query_map([], |row| {
            Ok(Session {
                id: row.get("id")?,
                start_time: row.get("start_time")?,
                end_time: row.get("end_time")?,
                stream_url: row.get("stream_url")?,
                stream_title: row.get("stream_title")?,
                total_messages: row.get("total_messages")?,
                total_revenue: row.get("total_revenue")?,
            })
        })?;

        let mut sessions = Vec::new();
        for session in session_iter {
            sessions.push(session?);
        }

        Ok(sessions)
    }

    /// セッションのメッセージを取得
    pub fn get_session_messages(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<GuiChatMessage>> {
        let sql = if let Some(limit) = limit {
            format!(
                "SELECT * FROM messages WHERE session_id = ?1 ORDER BY timestamp DESC LIMIT {}",
                limit
            )
        } else {
            "SELECT * FROM messages WHERE session_id = ?1 ORDER BY timestamp DESC".to_string()
        };

        let mut stmt = self.connection.prepare(&sql)?;
        let message_iter =
            stmt.query_map(params![session_id], |row| self.row_to_gui_message(row))?;

        let mut messages = Vec::new();
        for message in message_iter {
            messages.push(message?);
        }

        Ok(messages)
    }

    /// 上位貢献者を取得
    pub fn get_top_contributors(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<ViewerProfile>> {
        let mut stmt = self.connection.prepare(
            "SELECT vp.* FROM viewer_profiles vp
             INNER JOIN messages m ON vp.channel_id = m.channel_id
             WHERE m.session_id = ?1
             GROUP BY vp.channel_id
             ORDER BY vp.total_contribution DESC
             LIMIT ?2",
        )?;

        let profile_iter = stmt.query_map(params![session_id, limit], |row| {
            Ok(ViewerProfile {
                channel_id: row.get("channel_id")?,
                display_name: row.get("display_name")?,
                first_seen: row.get("first_seen")?,
                last_seen: row.get("last_seen")?,
                message_count: row.get("message_count")?,
                total_contribution: row.get("total_contribution")?,
                membership_level: row.get("membership_level")?,
                tags: row
                    .get::<_, Option<String>>("tags")?
                    .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
                    .unwrap_or_default(),
            })
        })?;

        let mut profiles = Vec::new();
        for profile in profile_iter {
            profiles.push(profile?);
        }

        Ok(profiles)
    }

    /// データベースの行をGUIメッセージに変換
    fn row_to_gui_message(&self, row: &Row) -> rusqlite::Result<GuiChatMessage> {
        let message_type_str: String = row.get("message_type")?;
        let amount: Option<f64> = row.get("amount")?;

        let message_type = match message_type_str.as_str() {
            "super-chat" => crate::gui::models::MessageType::SuperChat {
                amount: amount.map(|a| format!("¥{}", a)).unwrap_or_default(),
            },
            "super-sticker" => crate::gui::models::MessageType::SuperSticker {
                amount: amount.map(|a| format!("¥{}", a)).unwrap_or_default(),
            },
            "membership" | "membership-milestone" => crate::gui::models::MessageType::Membership { milestone_months: None },
            "system" => crate::gui::models::MessageType::System,
            _ => crate::gui::models::MessageType::Text,
        };

        let metadata_json: Option<String> = row.get("metadata")?;
        let metadata = if let Some(json) = metadata_json {
            serde_json::from_str(&json).ok()
        } else {
            None
        };

        let timestamp: String = row.get("timestamp")?;
        let db_id: i64 = row.get("id")?;

        Ok(GuiChatMessage {
            id: format!("db_{}", db_id),
            timestamp: timestamp.clone(),
            timestamp_usec: String::new(), // データベースにはマイクロ秒タイムスタンプは保存されない
            message_type,
            author: row.get("author")?,
            author_icon_url: None, // データベースにはアイコンURLは保存されない
            channel_id: row.get("channel_id")?,
            content: row.get("content")?,
            runs: Vec::new(),
            metadata,
            is_member: false,
            comment_count: None, // データベースからは回数情報は取得されない
        })
    }

    /// 金額文字列をデータベース用にパース（堅牢性強化版）
    fn parse_amount_for_db(&self, amount_str: &str) -> Option<f64> {
        Self::parse_amount(amount_str)
    }

    /// Extracts a floating point amount from locale-specific currency strings.
    fn parse_amount(amount_str: &str) -> Option<f64> {
        const MAX_LENGTH: usize = 50;

        let trimmed = amount_str.trim();
        if trimmed.is_empty() {
            tracing::debug!("Empty amount string provided");
            return None;
        }

        if trimmed.len() > MAX_LENGTH {
            tracing::warn!("Amount string too long ({}): {}", trimmed.len(), trimmed);
            return None;
        }

        if trimmed.contains('-') || (trimmed.contains('(') && trimmed.contains(')')) {
            tracing::warn!("Negative amount detected: {}", trimmed);
            return None;
        }

        let mut filtered = String::new();
        for ch in trimmed.chars() {
            match ch {
                '0'..='9' => filtered.push(ch),
                '.' | ',' => filtered.push(ch),
                '\'' | ' ' | ' ' | ' ' => {}
                _ => {}
            }
        }

        if filtered.is_empty() {
            tracing::debug!("No valid numeric characters in amount: {}", trimmed);
            return None;
        }

        let last_dot = filtered.rfind('.');
        let last_comma = filtered.rfind(',');
        let decimal_char = match (last_dot, last_comma) {
            (Some(d), Some(c)) => Some(if d > c { '.' } else { ',' }),
            (Some(d), None) => {
                let frac_len = filtered.len().saturating_sub(d + 1);
                if frac_len > 0 && frac_len <= 2 {
                    Some('.')
                } else {
                    None
                }
            }
            (None, Some(c)) => {
                let frac_len = filtered.len().saturating_sub(c + 1);
                if frac_len > 0 && frac_len <= 2 {
                    Some(',')
                } else {
                    None
                }
            }
            (None, None) => None,
        };

        let mut normalized = String::with_capacity(filtered.len());
        for ch in filtered.chars() {
            match ch {
                '0'..='9' => normalized.push(ch),
                '.' | ',' => {
                    if Some(ch) == decimal_char {
                        normalized.push('.');
                    }
                }
                _ => {}
            }
        }

        if normalized.is_empty() {
            return None;
        }

        match normalized.parse::<f64>() {
            Ok(amount) if amount >= 0.0 => Some(amount),
            Ok(amount) => {
                tracing::warn!("Negative amount detected after normalization: {}", amount);
                None
            }
            Err(e) => {
                tracing::warn!("Failed to parse normalized amount '{}': {}", normalized, e);
                None
            }
        }
    }

    /// 質問を保存
    pub fn save_question(&mut self, question: &Question) -> Result<i64> {
        let question_id = self
            .connection
            .prepare(
                "INSERT INTO questions 
             (message_id, session_id, detected_at, question_text, category, priority, confidence) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?
            .insert(params![
                question.message_id,
                question.session_id,
                question.detected_at.to_rfc3339(),
                question.question_text,
                question.category.as_str(),
                question.priority.clone() as i32,
                question.confidence,
            ])?;

        Ok(question_id)
    }

    /// セッションの質問一覧を取得
    pub fn get_session_questions(
        &self,
        session_id: &str,
        category: Option<&str>,
    ) -> Result<Vec<Question>> {
        let sql = if category.is_some() {
            "SELECT * FROM questions WHERE session_id = ?1 AND category = ?2 ORDER BY detected_at DESC"
        } else {
            "SELECT * FROM questions WHERE session_id = ?1 ORDER BY detected_at DESC"
        };

        let mut stmt = self.connection.prepare(sql)?;

        let question_iter = if let Some(cat) = category {
            stmt.query_map(params![session_id, cat], Self::row_to_question)?
        } else {
            stmt.query_map(params![session_id], Self::row_to_question)?
        };

        let mut questions = Vec::new();
        for question in question_iter {
            questions.push(question?);
        }

        Ok(questions)
    }

    /// データベースの行を質問に変換
    fn row_to_question(row: &Row) -> rusqlite::Result<Question> {
        Ok(Question {
            id: Some(row.get("id")?),
            message_id: row.get("message_id")?,
            session_id: row.get("session_id")?,
            detected_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("detected_at")?)
                .map_err(|_e| {
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "detected_at".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?
                .with_timezone(&Utc),
            question_text: row.get("question_text")?,
            category: match row.get::<_, String>("category")?.as_str() {
                "technical" => crate::chat_management::QuestionCategory::Technical,
                "general" => crate::chat_management::QuestionCategory::General,
                "request" => crate::chat_management::QuestionCategory::Request,
                "feedback" => crate::chat_management::QuestionCategory::Feedback,
                _ => crate::chat_management::QuestionCategory::Other,
            },
            priority: match row.get::<_, i32>("priority")? {
                3 => crate::chat_management::Priority::High,
                2 => crate::chat_management::Priority::Medium,
                _ => crate::chat_management::Priority::Low,
            },
            confidence: row.get("confidence")?,
            answered_at: row
                .get::<_, Option<String>>("answered_at")?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            answer_method: row
                .get::<_, Option<String>>("answer_method")?
                .and_then(|s| match s.as_str() {
                    "live_response" => Some(crate::chat_management::AnswerMethod::LiveResponse),
                    "template_response" => Some(
                        crate::chat_management::AnswerMethod::TemplateResponse("".to_string()),
                    ),
                    "ignored" => Some(crate::chat_management::AnswerMethod::Ignored),
                    "deferred" => Some(crate::chat_management::AnswerMethod::Deferred),
                    _ => None,
                }),
            notes: row.get("notes")?,
        })
    }

    // ========================================
    // 視聴者カスタム情報 CRUD操作
    // ========================================

    /// 視聴者カスタム情報を取得
    ///
    /// 配信者チャンネルIDと視聴者チャンネルIDで一意に特定される情報を取得する。
    pub fn get_viewer_custom_info(
        &self,
        broadcaster_channel_id: &str,
        viewer_channel_id: &str,
    ) -> Result<Option<ViewerCustomInfo>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, broadcaster_channel_id, viewer_channel_id, reading, notes, custom_data, created_at, updated_at
             FROM viewer_custom_info
             WHERE broadcaster_channel_id = ?1 AND viewer_channel_id = ?2",
        )?;

        let result = stmt.query_row(
            params![broadcaster_channel_id, viewer_channel_id],
            |row| {
                Ok(ViewerCustomInfo {
                    id: Some(row.get("id")?),
                    broadcaster_channel_id: row.get("broadcaster_channel_id")?,
                    viewer_channel_id: row.get("viewer_channel_id")?,
                    reading: row.get("reading")?,
                    notes: row.get("notes")?,
                    custom_data: row.get("custom_data")?,
                    created_at: row.get("created_at")?,
                    updated_at: row.get("updated_at")?,
                })
            },
        );

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 視聴者カスタム情報を作成または更新（Upsert）
    ///
    /// 既存のレコードがあれば更新し、なければ新規作成する。
    pub fn upsert_viewer_custom_info(&mut self, info: &ViewerCustomInfo) -> Result<i64> {
        self.connection.execute(
            "INSERT INTO viewer_custom_info
             (broadcaster_channel_id, viewer_channel_id, reading, notes, custom_data)
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

        Ok(self.connection.last_insert_rowid())
    }

    /// 配信者チャンネルに紐づくすべての視聴者カスタム情報を取得
    ///
    /// 配信接続時にキャッシュをプリロードするために使用する。
    /// キーは視聴者チャンネルID。
    pub fn get_all_viewer_custom_info_for_broadcaster(
        &self,
        broadcaster_channel_id: &str,
    ) -> Result<HashMap<String, ViewerCustomInfo>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, broadcaster_channel_id, viewer_channel_id, reading, notes, custom_data, created_at, updated_at
             FROM viewer_custom_info
             WHERE broadcaster_channel_id = ?1",
        )?;

        let rows = stmt.query_map(params![broadcaster_channel_id], |row| {
            Ok(ViewerCustomInfo {
                id: Some(row.get("id")?),
                broadcaster_channel_id: row.get("broadcaster_channel_id")?,
                viewer_channel_id: row.get("viewer_channel_id")?,
                reading: row.get("reading")?,
                notes: row.get("notes")?,
                custom_data: row.get("custom_data")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;

        let mut result = HashMap::new();
        for row in rows {
            let info = row?;
            result.insert(info.viewer_channel_id.clone(), info);
        }

        Ok(result)
    }

    /// 視聴者カスタム情報を削除
    pub fn delete_viewer_custom_info(
        &mut self,
        broadcaster_channel_id: &str,
        viewer_channel_id: &str,
    ) -> Result<bool> {
        let affected = self.connection.execute(
            "DELETE FROM viewer_custom_info WHERE broadcaster_channel_id = ?1 AND viewer_channel_id = ?2",
            params![broadcaster_channel_id, viewer_channel_id],
        )?;

        Ok(affected > 0)
    }
}

// ============================================================================
// スタンドアロン関数（rusqlite::Connectionを直接受け取る）
// use_live_chat.rs などからの呼び出し用
// ============================================================================

/// 視聴者カスタム情報を取得（スタンドアロン版）
pub fn get_viewer_custom_info(
    conn: &rusqlite::Connection,
    broadcaster_channel_id: &str,
    viewer_channel_id: &str,
) -> Result<Option<ViewerCustomInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, broadcaster_channel_id, viewer_channel_id, reading, notes, custom_data, created_at, updated_at
         FROM viewer_custom_info
         WHERE broadcaster_channel_id = ?1 AND viewer_channel_id = ?2",
    )?;

    let result = stmt.query_row(params![broadcaster_channel_id, viewer_channel_id], |row| {
        Ok(ViewerCustomInfo {
            id: Some(row.get(0)?),
            broadcaster_channel_id: row.get(1)?,
            viewer_channel_id: row.get(2)?,
            reading: row.get(3)?,
            notes: row.get(4)?,
            custom_data: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    });

    match result {
        Ok(info) => Ok(Some(info)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// 視聴者カスタム情報を保存または更新（スタンドアロン版）
pub fn upsert_viewer_custom_info(conn: &rusqlite::Connection, info: &ViewerCustomInfo) -> Result<i64> {
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

/// 指定した配信者の全視聴者カスタム情報を取得（スタンドアロン版）
pub fn get_all_viewer_custom_info_for_broadcaster(
    conn: &rusqlite::Connection,
    broadcaster_channel_id: &str,
) -> Result<HashMap<String, ViewerCustomInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, broadcaster_channel_id, viewer_channel_id, reading, notes, custom_data, created_at, updated_at
         FROM viewer_custom_info
         WHERE broadcaster_channel_id = ?1",
    )?;

    let rows = stmt.query_map(params![broadcaster_channel_id], |row| {
        Ok(ViewerCustomInfo {
            id: Some(row.get(0)?),
            broadcaster_channel_id: row.get(1)?,
            viewer_channel_id: row.get(2)?,
            reading: row.get(3)?,
            notes: row.get(4)?,
            custom_data: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    let mut result = HashMap::new();
    for row in rows {
        let info = row?;
        result.insert(info.viewer_channel_id.clone(), info);
    }

    Ok(result)
}

/// 視聴者カスタム情報を削除（スタンドアロン版）
pub fn delete_viewer_custom_info(
    conn: &rusqlite::Connection,
    broadcaster_channel_id: &str,
    viewer_channel_id: &str,
) -> Result<bool> {
    let affected = conn.execute(
        "DELETE FROM viewer_custom_info WHERE broadcaster_channel_id = ?1 AND viewer_channel_id = ?2",
        params![broadcaster_channel_id, viewer_channel_id],
    )?;

    Ok(affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

    fn next_test_id() -> (String, String) {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        (format!("test_{}", counter), counter.to_string())
    }

    fn create_test_message(
        author: &str,
        content: &str,
        message_type: crate::gui::models::MessageType,
    ) -> GuiChatMessage {
        let (id, timestamp_usec) = next_test_id();
        GuiChatMessage {
            id,
            timestamp: "12:00:00".to_string(),
            timestamp_usec,
            message_type,
            author: author.to_string(),
            author_icon_url: None,
            channel_id: "test_channel".to_string(),
            content: content.to_string(),
            runs: Vec::new(),
            metadata: None,
            is_member: false,
            comment_count: None,
        }
    }

    fn approx_eq(a: f64, b: f64) {
        assert!((a - b).abs() < 0.0001, "expected {} =~ {}", a, b);
    }

    #[test]
    fn test_parse_amount_locale_variants() {
        let cases = [
            ("€5,00", 5.00),
            ("R$ 1.234,56", 1234.56),
            ("US$1,234.56", 1234.56),
            ("1 234,56 PLN", 1234.56),
            ("CHF 1'234.50", 1234.50),
            ("CAD 1 234,56", 1234.56),
            ("¥500", 500.0),
            ("NT$1,000", 1000.0),
        ];

        for (input, expected) in cases {
            let parsed = LiscovDatabase::parse_amount(input)
                .expect(&format!("amount should parse: {}", input));
            approx_eq(parsed, expected);
        }

        assert!(LiscovDatabase::parse_amount("").is_none());
        assert!(LiscovDatabase::parse_amount("-5,00").is_none());
        assert!(LiscovDatabase::parse_amount("(5.00)").is_none());
        assert!(LiscovDatabase::parse_amount("abc").is_none());
    }

    #[test]
    fn test_database_creation() -> Result<()> {
        let db = LiscovDatabase::new_in_memory()?;
        assert_eq!(db.schema_version, 1);
        Ok(())
    }

    #[test]
    fn test_session_management() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;

        let session_id =
            db.create_session("https://youtube.com/watch?v=test", Some("Test Stream"))?;
        assert!(!session_id.is_empty());

        let sessions = db.get_sessions(Some(10))?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session_id);

        db.end_session(&session_id)?;
        let sessions = db.get_sessions(Some(10))?;
        assert!(sessions[0].end_time.is_some());

        Ok(())
    }

    #[test]
    fn test_message_storage() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;
        let session_id = db.create_session("https://youtube.com/watch?v=test", None)?;

        let message = create_test_message(
            "TestUser",
            "Thank you!",
            crate::gui::models::MessageType::SuperChat {
                amount: "¥100".to_string(),
            },
        );

        let message_id = db.save_message(&session_id, &message)?;
        assert!(message_id > 0);

        let messages = db.get_session_messages(&session_id, Some(10))?;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].author, "TestUser");

        Ok(())
    }

    #[test]
    fn test_create_session_with_invalid_url() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;

        // 非常に長いURLでのテスト
        let long_url = format!("https://youtube.com/watch?v={}", "x".repeat(1000));
        let session_id = db.create_session(&long_url, None)?;
        assert!(!session_id.is_empty());

        // 空のURLでのテスト
        let empty_session_id = db.create_session("", None)?;
        assert!(!empty_session_id.is_empty());

        Ok(())
    }

    #[test]
    fn test_session_operations_with_nonexistent_id() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;
        let fake_session_id = "nonexistent-session-id";

        // 存在しないセッションの終了を試行
        db.end_session(fake_session_id)?; // エラーにならないが何も起こらない

        // 存在しないセッションの統計更新を試行
        db.update_session_stats(fake_session_id)?; // エラーにならないが何も起こらない

        // 存在しないセッションのメッセージを取得
        let messages = db.get_session_messages(fake_session_id, Some(10))?;
        assert_eq!(messages.len(), 0);

        Ok(())
    }

    #[test]
    fn test_message_storage_edge_cases() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;
        let session_id = db.create_session("https://youtube.com/watch?v=test", None)?;

        // 空のメッセージコンテンツ
        let empty_message = create_test_message(
            "TestUser",
            "", // 空のコンテンツ
            crate::gui::models::MessageType::Text,
        );

        let empty_msg_id = db.save_message(&session_id, &empty_message)?;
        assert!(empty_msg_id > 0);

        // 非常に長いメッセージコンテンツ
        let long_content = "a".repeat(10000);
        let long_message = create_test_message(
            "TestUser",
            &long_content,
            crate::gui::models::MessageType::Text,
        );

        let long_msg_id = db.save_message(&session_id, &long_message)?;
        assert!(long_msg_id > 0);

        // 特殊文字を含むメッセージ
        let (id, timestamp_usec) = next_test_id();
        let special_message = GuiChatMessage {
            id,
            timestamp: "12:02:00".to_string(),
            timestamp_usec,
            message_type: crate::gui::models::MessageType::SuperChat {
                amount: "¥1000".to_string(),
            },
            author: "テストユーザー🎮".to_string(),
            author_icon_url: None,
            channel_id: "test123".to_string(),
            content: "🔥日本語メッセージ with special chars: \\n\\t\"'".to_string(),
            runs: Vec::new(),
            metadata: Some(crate::gui::models::MessageMetadata {
                amount: Some("¥1000".to_string()),
                badges: vec!["SuperChat".to_string()],
                badge_info: Vec::new(),
                color: Some("#ff0000".to_string()),
                is_moderator: false,
                is_verified: false,
                superchat_colors: None,
            }),
            is_member: true,
            comment_count: None,
        };

        let special_msg_id = db.save_message(&session_id, &special_message)?;
        assert!(special_msg_id > 0);

        // 全メッセージを取得して確認
        let all_messages = db.get_session_messages(&session_id, None)?;
        assert_eq!(all_messages.len(), 3);

        // 長いメッセージが正しく保存されているか確認
        let long_msg = all_messages
            .iter()
            .find(|m| m.content.len() > 5000)
            .unwrap();
        assert_eq!(long_msg.content, long_content);

        Ok(())
    }

    #[test]
    fn test_save_message_to_nonexistent_session() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;
        let fake_session_id = "nonexistent-session-id";

        let message = create_test_message(
            "TestUser",
            "Test message",
            crate::gui::models::MessageType::Text,
        );

        // 存在しないセッションへのメッセージ保存
        // 外部キー制約があれば失敗するが、現在の実装では成功する可能性がある
        let result = db.save_message(fake_session_id, &message);

        // エラーになるかメッセージIDが返されるかのどちらか
        match result {
            Ok(msg_id) => assert!(msg_id > 0),
            Err(_) => (), // 外部キー制約エラーの場合
        }

        Ok(())
    }

    #[test]
    fn test_database_schema_consistency() -> Result<()> {
        let db = LiscovDatabase::new_in_memory()?;

        // スキーマバージョンが正しく設定されているか確認
        assert_eq!(db.schema_version, 1);

        // データベース接続が有効か確認
        let mut stmt = db
            .connection
            .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")?;
        let table_count: i64 = stmt.query_row([], |row| row.get(0))?;

        // 期待されるテーブル数を確認（sessions, messages, viewer_profiles, questions, etc.）
        assert!(
            table_count >= 5,
            "Expected at least 5 tables, found {}",
            table_count
        );

        Ok(())
    }

    #[test]
    fn test_large_dataset_performance() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;
        let session_id = db.create_session("https://youtube.com/watch?v=perf_test", None)?;

        // 大量のメッセージを挿入してパフォーマンスをテスト
        let start_time = std::time::Instant::now();

        for i in 0..1000 {
            let (id, timestamp_usec) = next_test_id();
            let message = GuiChatMessage {
                id,
                timestamp: format!("12:{:02}:{:02}", i / 60, i % 60),
                timestamp_usec,
                message_type: if i % 10 == 0 {
                    crate::gui::models::MessageType::SuperChat {
                        amount: format!("¥{}", (i + 1) * 100),
                    }
                } else {
                    crate::gui::models::MessageType::Text
                },
                author: format!("User{}", i),
                author_icon_url: None,
                channel_id: format!("channel{}", i % 100),
                content: format!("Test message number {}", i),
                runs: Vec::new(),
                metadata: if i % 50 == 0 {
                    Some(crate::gui::models::MessageMetadata {
                        amount: Some(format!("¥{}", i * 10)),
                        badges: vec![format!("Badge{}", i)],
                        badge_info: Vec::new(),
                        color: Some("#0000ff".to_string()),
                        is_moderator: false,
                        is_verified: false,
                superchat_colors: None,
                    })
                } else {
                    None
                },
                is_member: i % 20 == 0,
                comment_count: None,
            };

            db.save_message(&session_id, &message)?;
        }

        let insert_duration = start_time.elapsed();
        println!("1000メッセージの挿入時間: {:?}", insert_duration);

        // 全メッセージの取得時間をテスト
        let fetch_start = std::time::Instant::now();
        let all_messages = db.get_session_messages(&session_id, None)?;
        let fetch_duration = fetch_start.elapsed();

        assert_eq!(all_messages.len(), 1000);
        println!("1000メッセージの取得時間: {:?}", fetch_duration);

        // パフォーマンスの期待値（あまり厳しくない）
        assert!(
            insert_duration.as_millis() < 5000,
            "メッセージ挿入が遅すぎます: {:?}",
            insert_duration
        );
        assert!(
            fetch_duration.as_millis() < 1000,
            "メッセージ取得が遅すぎます: {:?}",
            fetch_duration
        );

        Ok(())
    }

    #[test]
    fn test_concurrent_access_safety() -> Result<()> {
        use std::sync::{Arc, Mutex};
        use std::thread;

        // メモリ内データベースは単一接続のため、実際の同時アクセステストは制限される
        // ここでは基本的な排他制御の動作確認のみ行う

        let mut db = LiscovDatabase::new_in_memory()?;
        let session_id = db.create_session("https://youtube.com/watch?v=concurrent_test", None)?;

        // データベースを共有可能な形でラップ
        let db_mutex = Arc::new(Mutex::new(db));
        let session_id_clone = session_id.clone();

        let db_clone = Arc::clone(&db_mutex);
        let handle = thread::spawn(move || {
            let mut db_guard = db_clone.lock().unwrap();

            for i in 0..10 {
                let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
                let message = GuiChatMessage {
                    id: format!("test_{}", counter),
                    timestamp: format!("12:00:{:02}", i),
                    timestamp_usec: counter.to_string(),
                    message_type: crate::gui::models::MessageType::Text,
                    author: format!("ThreadUser{}", i),
                    author_icon_url: None,
                    channel_id: "thread_test".to_string(),
                    content: format!("Thread message {}", i),
                    runs: Vec::new(),
                    metadata: None,
                    is_member: false,
                    comment_count: None,
                };

                db_guard.save_message(&session_id_clone, &message).unwrap();
            }
        });

        // メインスレッドでも並行してメッセージを追加
        {
            let mut db_guard = db_mutex.lock().unwrap();
            for i in 10..20 {
                let (id, timestamp_usec) = next_test_id();
                let message = GuiChatMessage {
                    id,
                    timestamp: format!("12:00:{:02}", i),
                    timestamp_usec,
                    message_type: crate::gui::models::MessageType::Text,
                    author: format!("MainUser{}", i),
                    author_icon_url: None,
                    channel_id: "main_test".to_string(),
                    content: format!("Main message {}", i),
                    runs: Vec::new(),
                    metadata: None,
                    is_member: false,
                    comment_count: None,
                };

                db_guard.save_message(&session_id, &message)?;
            }
        }

        handle.join().unwrap();

        // 全メッセージが正しく挿入されたか確認
        let db_guard = db_mutex.lock().unwrap();
        let all_messages = db_guard.get_session_messages(&session_id, None)?;
        assert_eq!(all_messages.len(), 20);

        Ok(())
    }

    // ========================================
    // 視聴者カスタム情報のテスト
    // ========================================

    #[test]
    fn test_viewer_custom_info_create_and_read() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;

        let broadcaster_id = "UC_broadcaster_123";
        let viewer_id = "UC_viewer_456";

        // 新規作成
        let info = ViewerCustomInfo::new(broadcaster_id.to_string(), viewer_id.to_string())
            .with_reading("やまだたろう");

        db.upsert_viewer_custom_info(&info)?;

        // 読み取り
        let retrieved = db
            .get_viewer_custom_info(broadcaster_id, viewer_id)?
            .expect("情報が存在するはず");

        assert_eq!(retrieved.broadcaster_channel_id, broadcaster_id);
        assert_eq!(retrieved.viewer_channel_id, viewer_id);
        assert_eq!(retrieved.reading, Some("やまだたろう".to_string()));
        assert!(retrieved.id.is_some());
        assert!(retrieved.created_at.is_some());

        Ok(())
    }

    #[test]
    fn test_viewer_custom_info_upsert() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;

        let broadcaster_id = "UC_broadcaster_upsert";
        let viewer_id = "UC_viewer_upsert";

        // 最初の作成
        let info1 = ViewerCustomInfo::new(broadcaster_id.to_string(), viewer_id.to_string())
            .with_reading("最初の読み");

        db.upsert_viewer_custom_info(&info1)?;

        // 更新（Upsert）
        let info2 = ViewerCustomInfo::new(broadcaster_id.to_string(), viewer_id.to_string())
            .with_reading("更新後の読み")
            .with_notes("メモ追加");

        db.upsert_viewer_custom_info(&info2)?;

        // 確認
        let retrieved = db
            .get_viewer_custom_info(broadcaster_id, viewer_id)?
            .expect("情報が存在するはず");

        assert_eq!(retrieved.reading, Some("更新後の読み".to_string()));
        assert_eq!(retrieved.notes, Some("メモ追加".to_string()));

        Ok(())
    }

    #[test]
    fn test_viewer_custom_info_isolation_by_broadcaster() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;

        let broadcaster1 = "UC_broadcaster_A";
        let broadcaster2 = "UC_broadcaster_B";
        let viewer = "UC_common_viewer";

        // 同じ視聴者に対して異なる配信者で異なる読みを設定
        let info1 = ViewerCustomInfo::new(broadcaster1.to_string(), viewer.to_string())
            .with_reading("読みA");

        let info2 = ViewerCustomInfo::new(broadcaster2.to_string(), viewer.to_string())
            .with_reading("読みB");

        db.upsert_viewer_custom_info(&info1)?;
        db.upsert_viewer_custom_info(&info2)?;

        // 配信者ごとに異なる読みが取得できることを確認
        let from_a = db
            .get_viewer_custom_info(broadcaster1, viewer)?
            .expect("情報が存在するはず");
        let from_b = db
            .get_viewer_custom_info(broadcaster2, viewer)?
            .expect("情報が存在するはず");

        assert_eq!(from_a.reading, Some("読みA".to_string()));
        assert_eq!(from_b.reading, Some("読みB".to_string()));

        Ok(())
    }

    #[test]
    fn test_viewer_custom_info_bulk_read() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;

        let broadcaster = "UC_broadcaster_bulk";

        // 複数の視聴者情報を登録
        for i in 0..5 {
            let viewer = format!("UC_viewer_{}", i);
            let reading = format!("よみがな{}", i);
            let info =
                ViewerCustomInfo::new(broadcaster.to_string(), viewer).with_reading(reading);
            db.upsert_viewer_custom_info(&info)?;
        }

        // 一括取得
        let all_info = db.get_all_viewer_custom_info_for_broadcaster(broadcaster)?;

        assert_eq!(all_info.len(), 5);
        assert!(all_info.contains_key("UC_viewer_0"));
        assert!(all_info.contains_key("UC_viewer_4"));
        assert_eq!(
            all_info.get("UC_viewer_2").unwrap().reading,
            Some("よみがな2".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_viewer_custom_info_not_found() -> Result<()> {
        let db = LiscovDatabase::new_in_memory()?;

        let result = db.get_viewer_custom_info("nonexistent_broadcaster", "nonexistent_viewer")?;

        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_viewer_custom_info_delete() -> Result<()> {
        let mut db = LiscovDatabase::new_in_memory()?;

        let broadcaster = "UC_broadcaster_del";
        let viewer = "UC_viewer_del";

        // 作成
        let info = ViewerCustomInfo::new(broadcaster.to_string(), viewer.to_string())
            .with_reading("削除テスト");
        db.upsert_viewer_custom_info(&info)?;

        // 存在確認
        assert!(db.get_viewer_custom_info(broadcaster, viewer)?.is_some());

        // 削除
        let deleted = db.delete_viewer_custom_info(broadcaster, viewer)?;
        assert!(deleted);

        // 削除確認
        assert!(db.get_viewer_custom_info(broadcaster, viewer)?.is_none());

        // 存在しないものを削除
        let deleted_again = db.delete_viewer_custom_info(broadcaster, viewer)?;
        assert!(!deleted_again);

        Ok(())
    }
}
