-- 視聴者管理E2Eテスト用データのクリーンアップ
-- 使用方法: sqlite3 liscov.db < viewer_management_cleanup.sql

-- テスト用視聴者カスタム情報を削除
DELETE FROM viewer_custom_info WHERE broadcaster_channel_id LIKE 'UC_TEST_BROADCASTER_%';

-- テスト用視聴者プロフィールを削除
DELETE FROM viewer_profiles WHERE channel_id LIKE 'UC_TEST_VIEWER_%';

-- テスト用配信者プロフィールを削除
DELETE FROM broadcaster_profiles WHERE channel_id LIKE 'UC_TEST_BROADCASTER_%';
