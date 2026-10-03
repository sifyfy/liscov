-- Migration 007: 視聴者一覧（配信者ごと・最近アクティブな順）を並べ替えなしで引けるようにする
CREATE INDEX IF NOT EXISTS idx_viewer_profiles_last_seen ON viewer_profiles(broadcaster_channel_id, last_seen DESC);
