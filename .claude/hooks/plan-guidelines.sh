#!/bin/bash
# plan作成/実行スキル起動時に再発防止ガイドラインをリマインドする
set -euo pipefail

input=$(cat)
skill_name=$(echo "$input" | jq -r '.tool_input.skill // empty' 2>/dev/null)

# writing-plans または executing-plans スキルでなければスキップ
case "$skill_name" in
  *writing-plans*|*executing-plans*)
    ;;
  *)
    exit 0
    ;;
esac

cat <<'GUIDELINES'
{
  "decision": "allow",
  "systemMessage": "【計画作成ガイドライン — 再発防止策】\n\n以下のチェックリストを計画に適用してください:\n\n1. **内部整合性チェック**: 後続タスクが前のタスクの前提を壊さないか確認する。特に「所有権の移動」「データ構造の変更」が他タスクのアクセスパターンに影響しないか検証する。\n2. **1タスク = 1コミット**: 各タスクを独立してコミット可能な単位にする。複数タスクを1コミットにまとめない。\n3. **計画逸脱時のエスカレーション**: 実装中に計画と異なるアプローチを取る必要が生じた場合、サイレントにスタブ化せず、必ずシフィさんにエスカレーションする。\n4. **Codexによる計画レビュー**: 重要な設計判断を含む計画は、実装前にCodexのレビューを受けて内部矛盾を検出する。"
}
GUIDELINES
