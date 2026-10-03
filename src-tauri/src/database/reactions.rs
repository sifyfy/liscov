//! ライブリアクションの保存と集計（08_database.md「reactions テーブル」）

use crate::core::models::{ReactionSummary, ReactionUpdate};
use anyhow::Result;
use rusqlite::{Connection, params};

/// 1 回の更新を絵文字ごとに保存する（同じセッション・同じ更新は無視する）
pub fn save_reaction_update(
    conn: &Connection,
    session_id: &str,
    update: &ReactionUpdate,
) -> Result<()> {
    let mut stmt = conn.prepare_cached(
        "INSERT OR IGNORE INTO reactions
             (session_id, update_time_usec, duration_seconds, emoji, count)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for (emoji, count) in update.counts() {
        stmt.execute(params![
            session_id,
            update.update_time_usec(),
            update.duration_seconds(),
            emoji,
            count
        ])?;
    }
    Ok(())
}

/// その配信（video_id）の絵文字別累計と、`since_usec` 以降の更新を返す
pub fn get_reaction_summary(
    conn: &Connection,
    video_id: &str,
    since_usec: i64,
) -> Result<ReactionSummary> {
    let mut stmt = conn.prepare(
        "SELECT r.emoji, SUM(r.count)
         FROM reactions r
         JOIN sessions s ON r.session_id = s.id
         WHERE s.video_id = ?1
         GROUP BY r.emoji",
    )?;
    let totals = stmt
        .query_map(params![video_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;

    // 同じ時刻の更新が別セッション（同じ配信への同時接続）にあっても混ぜないよう、セッションごとにまとめる
    let mut stmt = conn.prepare(
        "SELECT r.session_id, r.update_time_usec, r.duration_seconds, r.emoji, r.count
         FROM reactions r
         JOIN sessions s ON r.session_id = s.id
         WHERE s.video_id = ?1 AND r.update_time_usec >= ?2
         ORDER BY r.update_time_usec, r.session_id",
    )?;
    let rows = stmt
        .query_map(params![video_id, since_usec], |row| {
            Ok((
                (
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, u32>(2)?,
                ),
                (row.get::<_, String>(3)?, row.get::<_, u32>(4)?),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let recent = rows
        .chunk_by(|(a, _), (b, _)| a == b)
        .filter_map(|group| {
            let (_, time, duration) = group[0].0;
            let counts = group
                .iter()
                .map(|(_, emoji_count)| emoji_count.clone())
                .collect();
            ReactionUpdate::new(time, duration, counts)
        })
        .collect();

    Ok(ReactionSummary { totals, recent })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{Database, create_session};
    use std::collections::BTreeMap;

    fn update(time: i64, duration: u32, pairs: &[(&str, u32)]) -> ReactionUpdate {
        let counts: BTreeMap<String, u32> =
            pairs.iter().map(|(e, c)| (e.to_string(), *c)).collect();
        ReactionUpdate::new(time, duration, counts).unwrap()
    }

    fn session(conn: &Connection, video_id: &str) -> String {
        let url = format!("https://www.youtube.com/watch?v={}", video_id);
        create_session(conn, Some(&url), None, Some("UC_bc"), Some("BC")).unwrap()
    }

    fn rows(conn: &Connection) -> Vec<(String, i64, u32, String, u32)> {
        let mut stmt = conn
            .prepare(
                "SELECT session_id, update_time_usec, duration_seconds, emoji, count
                 FROM reactions ORDER BY emoji",
            )
            .unwrap();
        stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    }

    // 08_database.md: 更新（2秒、❤3・🎉5）→ 絵文字ごとに 1 行
    #[tokio::test]
    async fn saves_one_row_per_emoji() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.connection().await;
        let s1 = session(&conn, "vid1");

        save_reaction_update(
            &conn,
            &s1,
            &update(1790422357025983, 2, &[("❤", 3), ("🎉", 5)]),
        )
        .unwrap();

        assert_eq!(
            rows(&conn),
            vec![
                (s1.clone(), 1790422357025983, 2, "❤".to_string(), 3),
                (s1.clone(), 1790422357025983, 2, "🎉".to_string(), 5),
            ]
        );
    }

    // 08_database.md: 同じセッションで同じ更新をもう一度保存しても増えない
    #[tokio::test]
    async fn same_update_saved_twice_is_ignored() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.connection().await;
        let s1 = session(&conn, "vid1");
        let u = update(100, 1, &[("❤", 3)]);

        save_reaction_update(&conn, &s1, &u).unwrap();
        save_reaction_update(&conn, &s1, &u).unwrap();

        assert_eq!(rows(&conn).len(), 1);
    }

    // 08_database.md: セッションを消すとリアクションも消える
    #[tokio::test]
    async fn deleting_session_cascades() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.connection().await;
        let s1 = session(&conn, "vid1");
        save_reaction_update(&conn, &s1, &update(100, 1, &[("❤", 3)])).unwrap();

        conn.execute("DELETE FROM sessions WHERE id = ?1", [&s1])
            .unwrap();

        assert!(rows(&conn).is_empty());
    }

    // 02_chat.md: 累計は配信（video_id）単位。同じ配信に再接続したら続きから数える
    #[tokio::test]
    async fn totals_span_sessions_of_same_video() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.connection().await;
        let first = session(&conn, "vid1");
        let reconnect = session(&conn, "vid1");
        let other = session(&conn, "vid2");
        save_reaction_update(&conn, &first, &update(100, 1, &[("❤", 3), ("🎉", 4)])).unwrap();
        save_reaction_update(&conn, &reconnect, &update(200, 1, &[("❤", 2)])).unwrap();
        save_reaction_update(&conn, &other, &update(300, 1, &[("😄", 9)])).unwrap();

        let summary = get_reaction_summary(&conn, "vid1", i64::MAX).unwrap();

        assert_eq!(
            summary.totals,
            BTreeMap::from([("❤".to_string(), 5), ("🎉".to_string(), 4)])
        );
        assert!(summary.recent.is_empty());
    }

    // 02_chat.md: /live/ の URL で接続したセッションも同じ配信として累計する
    #[tokio::test]
    async fn totals_include_sessions_connected_by_live_url() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.connection().await;
        let watch = session(&conn, "vid1");
        let live = create_session(
            &conn,
            Some("https://youtube.com/live/vid1?feature=share"),
            None,
            Some("UC_bc"),
            Some("BC"),
        )
        .unwrap();
        save_reaction_update(&conn, &watch, &update(100, 1, &[("❤", 3)])).unwrap();
        save_reaction_update(&conn, &live, &update(200, 1, &[("❤", 2)])).unwrap();

        let summary = get_reaction_summary(&conn, "vid1", 0).unwrap();

        assert_eq!(summary.totals, BTreeMap::from([("❤".to_string(), 5)]));
        assert_eq!(summary.recent.len(), 2);
    }

    // 02_chat.md: recent は指定時刻以降の更新を、更新ごとにまとめて古い順に返す
    #[tokio::test]
    async fn recent_returns_updates_since_given_time_in_order() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.connection().await;
        let s1 = session(&conn, "vid1");
        let old = update(1_000_000, 1, &[("❤", 1)]);
        let a = update(61_000_000, 2, &[("❤", 3), ("🎉", 5)]);
        let b = update(62_000_000, 1, &[("😄", 1)]);
        for u in [&b, &old, &a] {
            save_reaction_update(&conn, &s1, u).unwrap();
        }

        let summary = get_reaction_summary(&conn, "vid1", 2_000_000).unwrap();

        assert_eq!(summary.recent, vec![a, b]);
        assert_eq!(summary.totals.values().sum::<u64>(), 10);
    }

    #[tokio::test]
    async fn unknown_video_has_empty_summary() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.connection().await;
        assert_eq!(
            get_reaction_summary(&conn, "none", 0).unwrap(),
            ReactionSummary::default()
        );
    }
}
