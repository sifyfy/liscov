# Makefile - 開発者向け統一エントリポイント (SSoT インターフェース)
# このファイルは lint-and-hooks-setup スキルで生成
#
# SSoT 設計:
#   - 実コマンドの一次定義は各言語の標準タスクランナーに置く:
#     * Rust: cargo の直接呼び出し
#     * TS/Svelte: package.json scripts (npm run ...)
#   - lefthook (pre-commit) と GitHub Actions (CI) はこの Makefile を呼ぶだけ
#   - 同じコマンドを 3 箇所 (hook/CI/手元) に書かない。1 箇所更新すれば追従する
#
# 編集ルール: 新ターゲット追加時は help にも追記すること

.PHONY: help install lint fix format typecheck test secscan all clean
.PHONY: lint-rs lint-ts fix-rs fix-ts format-rs format-ts
.PHONY: typecheck-rs typecheck-ts test-rs test-ts secscan-rs secscan-ts
.PHONY: secscan-staged

# デフォルトターゲット: ヘルプ表示
help:
	@echo "==== 統合ターゲット ===="
	@echo "  make install     - 開発者環境セットアップ (mise + lefthook + npm)"
	@echo "  make lint        - 全言語 静的解析"
	@echo "  make fix         - 全言語 自動修正"
	@echo "  make format      - 全言語 format 適用"
	@echo "  make typecheck   - 全言語 型チェック"
	@echo "  make test        - 全言語 unit/integration test"
	@echo "  make secscan     - 依存脆弱性 + secret スキャン (フルスキャン)"
	@echo "  make all         - lint + typecheck + test + secscan"
	@echo "  make clean       - ビルド成果物の削除"
	@echo ""
	@echo "==== 言語別 (debug 用) ===="
	@echo "  make lint-rs / lint-ts"
	@echo "  make fix-rs / fix-ts"
	@echo "  make test-rs / test-ts"
	@echo "  make secscan-rs / secscan-ts"
	@echo ""
	@echo "==== pre-commit 用 (高速版) ===="
	@echo "  make secscan-staged - staged 変更だけ secret スキャン"

# ---- 環境セットアップ ----
install:
	mise install
	pnpm install
	lefthook install

# ---- 統合ターゲット ----
# gitleaks は secscan に集約。lint は静的解析のみ
lint: lint-rs lint-ts

fix: fix-rs fix-ts

format: format-rs format-ts

typecheck: typecheck-rs typecheck-ts

test: test-rs test-ts

# フルスキャン (CI 用): secret 全履歴 + 依存脆弱性
secscan: secscan-rs secscan-ts
	gitleaks detect --no-banner --redact

# 高速版 (pre-commit 用): staged 変更だけ secret スキャン
# 単一 commit では dep 脆弱性スキャンは意味薄いため除外
secscan-staged:
	gitleaks protect --staged --no-banner --redact

all: lint typecheck test secscan

# ---- Rust (Cargo workspace: src-tauri + crates/mock-server) ----
# lint: 初期は warning を許容 (既存違反 16件は段階的に解消する)
# TODO(quality): 既存違反解消後、`-- -D warnings` を再付与して warning 見逃しを防ぐ
lint-rs:
	cargo clippy --workspace --all-targets

fix-rs:
	cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged
	cargo fmt --all

format-rs:
	cargo fmt --all

# clippy が型チェック含むため通常不要。明示的に分けるなら cargo check
typecheck-rs:
	cargo check --workspace --all-targets

test-rs:
	cargo test --workspace

# cargo audit の subcommand 解決は mise shim 経由 (mise exec -- 必須)
secscan-rs:
	cargo audit

# ---- TypeScript / Svelte 5 ----
# 注: パッケージマネージャは pnpm に統一 (package.json の packageManager フィールド参照)
lint-ts:
	pnpm lint

fix-ts:
	pnpm format

format-ts:
	pnpm format

typecheck-ts:
	pnpm typecheck

test-ts:
	pnpm test:run

secscan-ts:
	pnpm secscan

# ---- Clean ----
clean:
	cargo clean
	pnpm store prune
	rm -rf node_modules/.cache build .svelte-kit playwright-report test-results
