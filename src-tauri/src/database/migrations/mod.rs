//! Database migration system
//!
//! Handles schema versioning and migrations to ensure the database
//! schema is always up-to-date with the application code.

use crate::core::models::extract_video_id;
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::collections::HashSet;

/// Migration definition
struct Migration {
    /// Unique name (used as identifier in schema_versions table)
    name: &'static str,
    /// SQL to execute for this migration
    sql: &'static str,
    /// SQL のあとに同じトランザクションで実行する既存データの埋め直し（SQL だけでは書けないもの）
    backfill: Option<fn(&Connection) -> Result<()>>,
}

/// All migrations in order of application
/// New migrations should be added to the end of this list
const MIGRATIONS: &[Migration] = &[
    Migration {
        name: "001_initial",
        sql: include_str!("001_initial.sql"),
        backfill: None,
    },
    Migration {
        name: "002_viewer_streams",
        sql: include_str!("002_viewer_streams.sql"),
        backfill: None,
    },
    Migration {
        name: "003_backfill_viewer_streams",
        sql: include_str!("003_backfill_viewer_streams.sql"),
        backfill: None,
    },
    Migration {
        name: "004_reactions",
        sql: include_str!("004_reactions.sql"),
        backfill: None,
    },
    Migration {
        name: "005_message_badges",
        sql: include_str!("005_message_badges.sql"),
        backfill: None,
    },
    Migration {
        name: "006_session_video_id",
        sql: include_str!("006_session_video_id.sql"),
        backfill: Some(backfill_session_video_id),
    },
];

/// Run all pending migrations
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Check for legacy database (old schema without version tracking)
    if is_legacy_database(conn)? {
        handle_legacy_database(conn)?;
    }

    // Ensure schema_versions table exists
    create_schema_versions_table(conn)?;

    apply_migrations(conn, MIGRATIONS)
}

/// 未適用のマイグレーションを順に適用する
///
/// 1 つのマイグレーションと schema_versions への記録を同じトランザクションで行う。
/// 途中の文で失敗したら全体が戻るので、ALTER TABLE ADD COLUMN のように 2 回実行できない文も
/// 半端に適用されたまま残らない（08_database.md「マイグレーション」）。
fn apply_migrations(conn: &Connection, migrations: &[Migration]) -> Result<()> {
    let applied = get_applied_migrations(conn)?;

    for migration in migrations {
        if applied.contains(migration.name) {
            continue;
        }
        tracing::info!("Applying migration: {}", migration.name);

        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(migration.sql)
            .with_context(|| format!("Failed to apply migration: {}", migration.name))?;
        if let Some(backfill) = migration.backfill {
            backfill(&tx)
                .with_context(|| format!("Failed to backfill migration: {}", migration.name))?;
        }
        record_migration(&tx, migration.name)?;
        tx.commit()?;

        tracing::info!("Migration applied successfully: {}", migration.name);
    }

    Ok(())
}

/// 既存セッションの video_id を stream_url から埋める（006、08_database.md sessions.video_id）
///
/// 接続時と同じ `extract_video_id` で取り出す。URL の解析を SQL で書き直すと、書き方の対応が食い違うため。
fn backfill_session_video_id(conn: &Connection) -> Result<()> {
    let sessions = conn
        .prepare("SELECT id, stream_url FROM sessions WHERE stream_url IS NOT NULL")?
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut update = conn.prepare("UPDATE sessions SET video_id = ?1 WHERE id = ?2")?;
    for (id, url) in sessions {
        if let Some(video_id) = extract_video_id(&url) {
            update.execute(params![video_id, id])?;
        }
    }
    Ok(())
}

/// Create the schema_versions table if it doesn't exist
fn create_schema_versions_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_versions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            applied_at TEXT DEFAULT CURRENT_TIMESTAMP
        );",
    )?;
    Ok(())
}

/// Get the set of already applied migration names
fn get_applied_migrations(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT name FROM schema_versions")?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<HashSet<_>, _>>()?;
    Ok(names)
}

/// Record a migration as applied
fn record_migration(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("INSERT INTO schema_versions (name) VALUES (?1)", [name])?;
    Ok(())
}

/// Check if this is a legacy database (has tables but no schema_versions)
fn is_legacy_database(conn: &Connection) -> Result<bool> {
    // Check if schema_versions table exists
    let has_schema_versions: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_versions'",
        [],
        |row| row.get(0),
    )?;

    if has_schema_versions {
        return Ok(false);
    }

    // Check if viewer_profiles table exists (indicates legacy DB)
    let has_viewer_profiles: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='viewer_profiles'",
        [],
        |row| row.get(0),
    )?;

    Ok(has_viewer_profiles)
}

/// Handle legacy database migration
///
/// Legacy databases have the old schema where viewer_profiles uses
/// channel_id as primary key without broadcaster_channel_id.
fn handle_legacy_database(conn: &Connection) -> Result<()> {
    tracing::warn!("Detected legacy database schema. Attempting migration...");

    // Check if viewer_profiles has the old schema (no broadcaster_channel_id column)
    let has_broadcaster_column: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('viewer_profiles') WHERE name='broadcaster_channel_id'",
        [],
        |row| row.get(0),
    )?;

    if has_broadcaster_column {
        // Already has the new schema, just needs schema_versions tracking
        tracing::info!("Legacy database already has new schema, adding version tracking");
        create_schema_versions_table(conn)?;
        record_migration(conn, "001_initial")?;
        return Ok(());
    }

    // Need to migrate from old schema to new schema
    tracing::info!("Migrating viewer_profiles from old schema to new schema");

    // Start a transaction for the migration
    conn.execute_batch("BEGIN TRANSACTION;")?;

    let result = migrate_viewer_profiles_schema(conn);

    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT;")?;
            tracing::info!("Legacy database migration completed successfully");

            // Record the migration
            create_schema_versions_table(conn)?;
            record_migration(conn, "001_initial")?;

            Ok(())
        }
        Err(e) => {
            conn.execute_batch("ROLLBACK;")?;
            tracing::error!("Legacy database migration failed: {}", e);
            Err(e)
        }
    }
}

/// Migrate viewer_profiles from old schema to new schema
fn migrate_viewer_profiles_schema(conn: &Connection) -> Result<()> {
    // 1. Drop old indexes and triggers that reference the old schema
    conn.execute_batch(
        "DROP INDEX IF EXISTS idx_viewer_profiles_message_count;
         DROP INDEX IF EXISTS idx_viewer_profiles_contribution;
         DROP TRIGGER IF EXISTS update_viewer_profiles_timestamp;",
    )?;

    // 2. Rename old table
    conn.execute_batch("ALTER TABLE viewer_profiles RENAME TO viewer_profiles_old;")?;

    // 3. Create new table with correct schema
    conn.execute_batch(
        "CREATE TABLE viewer_profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            broadcaster_channel_id TEXT NOT NULL,
            channel_id TEXT NOT NULL,
            display_name TEXT NOT NULL,
            first_seen TEXT NOT NULL,
            last_seen TEXT NOT NULL,
            message_count INTEGER DEFAULT 0,
            total_contribution REAL DEFAULT 0.0,
            membership_level TEXT,
            tags TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(broadcaster_channel_id, channel_id)
        );",
    )?;

    // 4. Migrate data - use a placeholder broadcaster_id for orphaned viewers
    // Try to match viewers to broadcasters via sessions/messages
    conn.execute_batch(
        "INSERT INTO viewer_profiles (
            broadcaster_channel_id, channel_id, display_name, first_seen, last_seen,
            message_count, total_contribution, membership_level, tags, created_at, updated_at
        )
        SELECT
            COALESCE(
                (SELECT DISTINCT s.broadcaster_channel_id
                 FROM messages m
                 JOIN sessions s ON m.session_id = s.id
                 WHERE m.channel_id = vp.channel_id
                 AND s.broadcaster_channel_id IS NOT NULL
                 LIMIT 1),
                'UNKNOWN_BROADCASTER'
            ) as broadcaster_channel_id,
            vp.channel_id,
            vp.display_name,
            vp.first_seen,
            vp.last_seen,
            vp.message_count,
            vp.total_contribution,
            vp.membership_level,
            vp.tags,
            vp.created_at,
            vp.updated_at
        FROM viewer_profiles_old vp;",
    )?;

    // 5. Migrate viewer_custom_info if it exists and has old schema
    let has_custom_info: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='viewer_custom_info'",
        [],
        |row| row.get(0),
    )?;

    if has_custom_info {
        // Check if it has old schema (broadcaster_channel_id + viewer_channel_id)
        let has_old_custom_info_schema: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('viewer_custom_info') WHERE name='broadcaster_channel_id'",
            [],
            |row| row.get(0),
        )?;

        if has_old_custom_info_schema {
            // Migrate custom info
            conn.execute_batch("ALTER TABLE viewer_custom_info RENAME TO viewer_custom_info_old;")?;

            conn.execute_batch(
                "CREATE TABLE viewer_custom_info (
                    viewer_profile_id INTEGER PRIMARY KEY,
                    reading TEXT,
                    notes TEXT,
                    custom_data TEXT,
                    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (viewer_profile_id) REFERENCES viewer_profiles(id) ON DELETE CASCADE
                );",
            )?;

            conn.execute_batch(
                "INSERT INTO viewer_custom_info (viewer_profile_id, reading, notes, custom_data, created_at, updated_at)
                SELECT
                    vp.id,
                    vci.reading,
                    vci.notes,
                    vci.custom_data,
                    vci.created_at,
                    vci.updated_at
                FROM viewer_custom_info_old vci
                JOIN viewer_profiles vp ON vp.channel_id = vci.viewer_channel_id
                    AND vp.broadcaster_channel_id = vci.broadcaster_channel_id;"
            )?;

            conn.execute_batch("DROP TABLE viewer_custom_info_old;")?;
        }
    }

    // 6. Drop old table
    conn.execute_batch("DROP TABLE viewer_profiles_old;")?;

    // 7. Create new indexes
    conn.execute_batch(
        "CREATE INDEX idx_viewer_profiles_broadcaster ON viewer_profiles(broadcaster_channel_id);
         CREATE INDEX idx_viewer_profiles_message_count ON viewer_profiles(broadcaster_channel_id, message_count DESC);
         CREATE INDEX idx_viewer_profiles_contribution ON viewer_profiles(broadcaster_channel_id, total_contribution DESC);"
    )?;

    // 8. Create trigger
    conn.execute_batch(
        "CREATE TRIGGER update_viewer_profiles_timestamp
            AFTER UPDATE ON viewer_profiles
            BEGIN
                UPDATE viewer_profiles SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
            END;",
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_database_migration() {
        let conn = Connection::open_in_memory().unwrap();

        // Run migrations on fresh database
        run_migrations(&conn).unwrap();

        // Verify schema_versions exists and has the migration
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_versions WHERE name = '001_initial'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Verify tables were created
        let has_viewer_profiles: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='viewer_profiles'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(has_viewer_profiles);
    }

    #[test]
    fn test_idempotent_migration() {
        let conn = Connection::open_in_memory().unwrap();

        // Run migrations twice
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        // Should have exactly the number of migrations (no duplicates)
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_versions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, MIGRATIONS.len() as i64);
    }

    #[test]
    fn test_legacy_database_detection() {
        let conn = Connection::open_in_memory().unwrap();

        // Create old schema without schema_versions
        conn.execute_batch(
            "CREATE TABLE viewer_profiles (
                channel_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                message_count INTEGER DEFAULT 0,
                total_contribution REAL DEFAULT 0.0,
                membership_level TEXT,
                tags TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .unwrap();

        // Should detect as legacy
        assert!(is_legacy_database(&conn).unwrap());
    }
    fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info(?1) WHERE name = ?2",
            [table, column],
            |row| row.get(0),
        )
        .unwrap()
    }

    // 08_database.md「マイグレーション」: 途中の文で失敗したら全体を戻し、適用済みにしない
    #[test]
    fn failed_migration_is_rolled_back() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (id INTEGER);").unwrap();
        create_schema_versions_table(&conn).unwrap();
        let broken = [Migration {
            name: "999_broken",
            sql: "ALTER TABLE t ADD COLUMN a TEXT; ALTER TABLE missing ADD COLUMN b TEXT;",
            backfill: None,
        }];

        assert!(apply_migrations(&conn, &broken).is_err());

        assert!(
            !has_column(&conn, "t", "a"),
            "失敗したマイグレーションの列が残っている"
        );
        assert!(
            !get_applied_migrations(&conn)
                .unwrap()
                .contains("999_broken")
        );
        // 直したものを流し直せる（半端に適用されていないので ALTER が重複しない）
        let fixed = [Migration {
            name: "999_broken",
            sql: "ALTER TABLE t ADD COLUMN a TEXT;",
            backfill: None,
        }];
        apply_migrations(&conn, &fixed).unwrap();
        assert!(has_column(&conn, "t", "a"));
    }

    // 08_database.md messages テーブル: 005 でスパチャの色とバッジの列を足す
    #[test]
    fn migration_005_adds_color_and_badge_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        for column in ["superchat_color", "is_moderator", "is_verified", "badges"] {
            assert!(has_column(&conn, "messages", column), "{column}");
        }
    }

    // 08_database.md sessions.video_id: 006 は既存セッションの stream_url から video_id を埋める
    #[test]
    fn migration_006_backfills_video_id_from_stream_url() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema_versions_table(&conn).unwrap();
        let before_006 = MIGRATIONS
            .iter()
            .position(|m| m.name == "006_session_video_id")
            .unwrap();
        apply_migrations(&conn, &MIGRATIONS[..before_006]).unwrap();
        let urls = [
            ("watch", Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")),
            ("live", Some("https://www.youtube.com/live/dQw4w9WgXcQ")),
            (
                "share",
                Some("https://youtube.com/live/dQw4w9WgXcQ?feature=share"),
            ),
            ("short", Some("https://youtu.be/dQw4w9WgXcQ")),
            ("null", None),
        ];
        for (id, url) in urls {
            conn.execute(
                "INSERT INTO sessions (id, start_time, stream_url) VALUES (?1, '2026-01-01T00:00:00Z', ?2)",
                rusqlite::params![id, url],
            )
            .unwrap();
        }

        run_migrations(&conn).unwrap();

        for (id, url) in urls {
            let video_id: Option<String> = conn
                .query_row("SELECT video_id FROM sessions WHERE id = ?1", [id], |r| {
                    r.get(0)
                })
                .unwrap();
            let expected = url.map(|_| "dQw4w9WgXcQ");
            assert_eq!(video_id.as_deref(), expected, "{id}");
        }
    }
}
