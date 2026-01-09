-- 視聴者管理E2Eテスト用データ
-- 使用方法: sqlite3 liscov.db < viewer_management_test_data.sql

-- 配信者プロフィール
INSERT OR REPLACE INTO broadcaster_profiles (channel_id, channel_name, handle) VALUES
    ('UC_TEST_BROADCASTER_001', 'テスト配信者A', '@test_broadcaster_a'),
    ('UC_TEST_BROADCASTER_002', 'テスト配信者B', '@test_broadcaster_b'),
    ('UC_TEST_BROADCASTER_DEL', '削除テスト用配信者', '@delete_broadcaster');

-- 視聴者プロフィール
INSERT OR REPLACE INTO viewer_profiles (channel_id, display_name, first_seen, last_seen, message_count, total_contribution, membership_level, tags) VALUES
    ('UC_TEST_VIEWER_001', 'テスト視聴者1', '2025-01-01T00:00:00Z', '2025-01-10T00:00:00Z', 100, 0.0, NULL, 'タグA,タグB'),
    ('UC_TEST_VIEWER_002', 'テスト視聴者2_長い名前のユーザーです', '2025-01-02T00:00:00Z', '2025-01-10T00:00:00Z', 50, 0.0, 'メンバー', 'タグC'),
    ('UC_TEST_VIEWER_003', 'テスト視聴者3', '2025-01-03T00:00:00Z', '2025-01-10T00:00:00Z', 25, 0.0, NULL, NULL),
    ('UC_TEST_VIEWER_004', '削除テスト用視聴者', '2025-01-04T00:00:00Z', '2025-01-10T00:00:00Z', 10, 0.0, NULL, NULL),
    ('UC_TEST_VIEWER_005', '編集テスト用視聴者', '2025-01-05T00:00:00Z', '2025-01-10T00:00:00Z', 5, 0.0, NULL, NULL);

-- 視聴者カスタム情報（配信者Aに紐づく）
INSERT OR REPLACE INTO viewer_custom_info (broadcaster_channel_id, viewer_channel_id, reading, notes) VALUES
    ('UC_TEST_BROADCASTER_001', 'UC_TEST_VIEWER_001', 'てすとしちょうしゃいち', 'テストメモ1'),
    ('UC_TEST_BROADCASTER_001', 'UC_TEST_VIEWER_002', 'てすとしちょうしゃに', NULL),
    ('UC_TEST_BROADCASTER_001', 'UC_TEST_VIEWER_003', NULL, 'メモのみ'),
    ('UC_TEST_BROADCASTER_001', 'UC_TEST_VIEWER_004', 'さくじょてすと', '削除されるデータ'),
    ('UC_TEST_BROADCASTER_001', 'UC_TEST_VIEWER_005', 'へんしゅうまえ', '編集前のメモ');

-- 視聴者カスタム情報（配信者Bに紐づく - 別配信者のデータ）
INSERT OR REPLACE INTO viewer_custom_info (broadcaster_channel_id, viewer_channel_id, reading, notes) VALUES
    ('UC_TEST_BROADCASTER_002', 'UC_TEST_VIEWER_001', 'べつのよみかた', '配信者Bでの情報');

-- 視聴者カスタム情報（削除テスト用配信者に紐づく）
INSERT OR REPLACE INTO viewer_custom_info (broadcaster_channel_id, viewer_channel_id, reading, notes) VALUES
    ('UC_TEST_BROADCASTER_DEL', 'UC_TEST_VIEWER_001', 'さくじょよう1', '削除される視聴者情報1'),
    ('UC_TEST_BROADCASTER_DEL', 'UC_TEST_VIEWER_002', 'さくじょよう2', '削除される視聴者情報2');
