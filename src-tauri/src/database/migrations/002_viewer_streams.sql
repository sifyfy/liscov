-- Migration 002: Viewer Streams
-- Tracks which streams each viewer has commented on.
-- Enables first-time viewer detection by comparing the oldest video_id
-- with the current stream's video_id.

CREATE TABLE IF NOT EXISTS viewer_streams (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    viewer_profile_id INTEGER NOT NULL,
    video_id TEXT NOT NULL,
    first_comment_at TEXT NOT NULL,
    last_comment_at TEXT NOT NULL,
    message_count INTEGER DEFAULT 1,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (viewer_profile_id) REFERENCES viewer_profiles(id) ON DELETE CASCADE,
    UNIQUE(viewer_profile_id, video_id)
);

CREATE INDEX IF NOT EXISTS idx_viewer_streams_video ON viewer_streams(video_id);
CREATE INDEX IF NOT EXISTS idx_viewer_streams_profile ON viewer_streams(viewer_profile_id);
CREATE INDEX IF NOT EXISTS idx_viewer_streams_first_comment ON viewer_streams(viewer_profile_id, first_comment_at ASC);
