-- Migration 005: スーパーチャットの色と発言者のバッジ
-- 色は段階の判定（07_revenue.md）、バッジはエクスポート（07_revenue.md）に使う。
-- 005 より前の行は NULL のまま（不明）。See: docs/specs/08_database.md

ALTER TABLE messages ADD COLUMN superchat_color TEXT;
ALTER TABLE messages ADD COLUMN is_moderator INTEGER;
ALTER TABLE messages ADD COLUMN is_verified INTEGER;
ALTER TABLE messages ADD COLUMN badges TEXT;
