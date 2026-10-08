/**
 * E2Eテスト用プリビルドスクリプト
 *
 * 毎回フロントエンドとRustバイナリをビルドする。成果物の有無や mtime では判定しない。
 * 理由: 存在チェックだけでは変更後も古い成果物のまま E2E が走り、壊した本番経路でも通ってしまった。
 * 変更が無いときのコストは cargo が約1秒 (インクリメンタル)、pnpm build が約13秒 (2026-10-09 実測)。
 *
 * 注: debug ビルドの exe はフロントエンドを埋め込まない (custom-protocol 無しでは devUrl を読む)。
 * E2E では test-helpers.ts が build/ を devUrl のポート (5173) で配信するので、
 * フロントエンドだけ変えたときに exe を作り直す必要は無い。
 */
import { execSync } from 'child_process';
import * as path from 'path';

const PROJECT_DIR = path.resolve(import.meta.dirname, '..', '..');

console.log('[e2e-build] Building frontend...');
execSync('pnpm build', { cwd: PROJECT_DIR, stdio: 'inherit' });

// mock_server は別 workspace member (crates/mock-server) のため --workspace で両方ビルドする
console.log('[e2e-build] Building Rust binaries...');
execSync('cargo build --workspace', { cwd: PROJECT_DIR, stdio: 'inherit' });

console.log('[e2e-build] All artifacts ready.');
