#!/usr/bin/env bash
# flake-hunt.sh
# cargo test を N 回実行し、各回の全ログをファイルに保存する。
# 「grep で証拠喪失」を構造的に防ぐためだけのラッパー。
#
# Usage:
#   bash <skill_path>/scripts/flake-hunt.sh <iterations> <session_tag> \
#        [--pre-cmd <shell>] -- <cargo-test-args...>
#
# Example:
#   bash .claude/skills/flake-hunt/scripts/flake-hunt.sh 20 allbins -- \
#       --manifest-path src-tauri/Cargo.toml
#   bash .claude/skills/flake-hunt/scripts/flake-hunt.sh 100 libhunt -- \
#       --manifest-path src-tauri/Cargo.toml --lib
#   bash .claude/skills/flake-hunt/scripts/flake-hunt.sh 20 cov_then -- \
#       --pre-cmd "pnpm test:coverage >/dev/null 2>&1" \
#       --manifest-path src-tauri/Cargo.toml --lib
#
# Output:
#   .tmp/flake-runs/<tag>/run-N.log    各 iter の cargo test 全文ログ
#
# 失敗判定: cargo test の exit code 非0 または `test result.*FAILED` 行を含む。
# 失敗ログがあれば標準出力で `iter N: FAIL (...) → <path>` を 1 行ずつ表示し、
# 最後に集計を出す。呼び出し側はサマリのみ受け取り、必要に応じて該当 log を Read で確認する。
set -uo pipefail

N="${1:-}"
TAG="${2:-}"
if [ -z "$N" ] || [ -z "$TAG" ]; then
  echo "usage: $0 <iter> <tag> [--pre-cmd <shell>] -- <cargo-test-args>" >&2
  exit 2
fi
shift 2

PRE_CMD=""
while [ $# -gt 0 ]; do
  case "$1" in
    --pre-cmd) PRE_CMD="${2:-}"; shift 2 ;;
    --) shift; break ;;
    *) break ;;
  esac
done

OUT_DIR=".tmp/flake-runs/${TAG}"
mkdir -p "$OUT_DIR"

fails=0
for i in $(seq 1 "$N"); do
  LOG="$OUT_DIR/run-$i.log"
  if [ -n "$PRE_CMD" ]; then
    eval "$PRE_CMD"
  fi
  cargo test --no-fail-fast "$@" > "$LOG" 2>&1
  rc=$?
  if [ "$rc" -ne 0 ] || grep -qE "^test result.*(FAILED|failed; [1-9])" "$LOG"; then
    fails=$((fails + 1))
    reason="$([ "$rc" -ne 0 ] && echo "exit=$rc" || echo "result=FAILED")"
    echo "iter $i: FAIL ($reason) → $LOG"
  fi
done
echo "TOTAL: $N iters, $fails failed, logs in $OUT_DIR"
