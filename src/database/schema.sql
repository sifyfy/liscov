-- liscov データベーススキーマ定義
-- 作成日: 2025-01-31
-- バージョン: 1.0

-- セッションテーブル：配信セッション情報を管理
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    start_time TEXT NOT NULL,
    end_time TEXT,
    stream_url TEXT,
    stream_title TEXT,
    total_messages INTEGER DEFAULT 0,
    total_revenue REAL DEFAULT 0.0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- メッセージテーブル：チャットメッセージを管理
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    author TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    content TEXT NOT NULL,
    message_type TEXT NOT NULL,
    amount REAL,
    metadata TEXT, -- JSON形式でメタデータを保存
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- 視聴者プロフィールテーブル：視聴者の情報と統計を管理
CREATE TABLE IF NOT EXISTS viewer_profiles (
    channel_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    message_count INTEGER DEFAULT 0,
    total_contribution REAL DEFAULT 0.0,
    membership_level TEXT,
    tags TEXT, -- カンマ区切りでタグを保存
    behavior_stats TEXT, -- JSON形式で行動統計を保存
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- 質問テーブル：検出された質問を管理
CREATE TABLE IF NOT EXISTS questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id INTEGER NOT NULL,
    session_id TEXT NOT NULL,
    detected_at TEXT NOT NULL,
    question_text TEXT NOT NULL,
    category TEXT NOT NULL,
    priority INTEGER DEFAULT 0,
    confidence REAL DEFAULT 0.0,
    answered_at TEXT,
    answer_method TEXT,
    notes TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- 収益分析テーブル：時間別収益データを管理
CREATE TABLE IF NOT EXISTS hourly_revenue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    hour_timestamp TEXT NOT NULL,
    super_chat_amount REAL DEFAULT 0.0,
    membership_count INTEGER DEFAULT 0,
    message_count INTEGER DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, hour_timestamp)
);

-- 貢献者統計テーブル：セッション別貢献者データ
CREATE TABLE IF NOT EXISTS contributor_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    contribution_amount REAL DEFAULT 0.0,
    contribution_count INTEGER DEFAULT 0,
    last_contribution TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, channel_id)
);

-- インデックス作成：パフォーマンス向上のため
CREATE INDEX IF NOT EXISTS idx_messages_session_timestamp ON messages(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_channel_id ON messages(channel_id);
CREATE INDEX IF NOT EXISTS idx_messages_type ON messages(message_type);
CREATE INDEX IF NOT EXISTS idx_questions_session ON questions(session_id);
CREATE INDEX IF NOT EXISTS idx_questions_category ON questions(category);
CREATE INDEX IF NOT EXISTS idx_hourly_revenue_session ON hourly_revenue(session_id);
CREATE INDEX IF NOT EXISTS idx_contributor_stats_session ON contributor_stats(session_id);

-- トリガー：updated_atの自動更新
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

CREATE TRIGGER IF NOT EXISTS update_contributor_stats_timestamp 
    AFTER UPDATE ON contributor_stats
    BEGIN
        UPDATE contributor_stats SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
    END; 