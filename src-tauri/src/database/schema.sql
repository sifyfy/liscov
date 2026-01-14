-- Liscov Database Schema

-- Sessions table - track streaming sessions
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    start_time TEXT NOT NULL,
    end_time TEXT,
    stream_url TEXT,
    stream_title TEXT,
    broadcaster_channel_id TEXT,
    broadcaster_name TEXT,
    total_messages INTEGER DEFAULT 0,
    total_revenue REAL DEFAULT 0.0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Messages table - store chat messages
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    timestamp_usec TEXT NOT NULL,
    author TEXT NOT NULL,
    author_icon_url TEXT,
    channel_id TEXT NOT NULL,
    content TEXT NOT NULL,
    message_type TEXT NOT NULL,
    amount TEXT,
    is_member INTEGER DEFAULT 0,
    metadata TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_messages_session_timestamp ON messages(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_channel_id ON messages(channel_id);
CREATE INDEX IF NOT EXISTS idx_messages_type ON messages(message_type);
CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_unique ON messages(session_id, message_id);

-- Viewer profiles table - global viewer information
CREATE TABLE IF NOT EXISTS viewer_profiles (
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
);

CREATE INDEX IF NOT EXISTS idx_viewer_profiles_message_count ON viewer_profiles(message_count DESC);
CREATE INDEX IF NOT EXISTS idx_viewer_profiles_contribution ON viewer_profiles(total_contribution DESC);

-- Viewer custom info table - broadcaster-specific viewer information
CREATE TABLE IF NOT EXISTS viewer_custom_info (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    broadcaster_channel_id TEXT NOT NULL,
    viewer_channel_id TEXT NOT NULL,
    reading TEXT,
    notes TEXT,
    custom_data TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(broadcaster_channel_id, viewer_channel_id)
);

CREATE INDEX IF NOT EXISTS idx_viewer_custom_info_broadcaster ON viewer_custom_info(broadcaster_channel_id);
CREATE INDEX IF NOT EXISTS idx_viewer_custom_info_lookup ON viewer_custom_info(broadcaster_channel_id, viewer_channel_id);

-- Broadcaster profiles table
CREATE TABLE IF NOT EXISTS broadcaster_profiles (
    channel_id TEXT PRIMARY KEY,
    channel_name TEXT,
    handle TEXT,
    thumbnail_url TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Triggers for automatic timestamp updates
CREATE TRIGGER IF NOT EXISTS update_sessions_timestamp
    AFTER UPDATE ON sessions
    BEGIN
        UPDATE sessions SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
    END;

CREATE TRIGGER IF NOT EXISTS update_viewer_profiles_timestamp
    AFTER UPDATE ON viewer_profiles
    BEGIN
        UPDATE viewer_profiles SET updated_at = CURRENT_TIMESTAMP WHERE channel_id = NEW.channel_id;
    END;

CREATE TRIGGER IF NOT EXISTS update_viewer_custom_info_timestamp
    AFTER UPDATE ON viewer_custom_info
    BEGIN
        UPDATE viewer_custom_info SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
    END;

CREATE TRIGGER IF NOT EXISTS update_broadcaster_profiles_timestamp
    AFTER UPDATE ON broadcaster_profiles
    BEGIN
        UPDATE broadcaster_profiles SET updated_at = CURRENT_TIMESTAMP WHERE channel_id = NEW.channel_id;
    END;
