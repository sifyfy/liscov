-- Migration 004: ライブリアクション
-- 1 回の更新（emojiFountainDataEntity）の絵文字ごとに 1 行。See: docs/specs/08_database.md

CREATE TABLE IF NOT EXISTS reactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    update_time_usec INTEGER NOT NULL,
    duration_seconds INTEGER NOT NULL,
    emoji TEXT NOT NULL,
    count INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, update_time_usec, emoji)
);

CREATE INDEX IF NOT EXISTS idx_reactions_session_time ON reactions(session_id, update_time_usec);
