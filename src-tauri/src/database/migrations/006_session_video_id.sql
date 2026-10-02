-- Migration 006: 同じ配信のセッションを video_id で引けるようにする
-- 既存行の video_id は SQL のあとに Rust 側（backfill_session_video_id）で stream_url から埋める
ALTER TABLE sessions ADD COLUMN video_id TEXT;
CREATE INDEX IF NOT EXISTS idx_sessions_video_id ON sessions(video_id);
