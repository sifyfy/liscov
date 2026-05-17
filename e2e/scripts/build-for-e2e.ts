/**
 * E2Eテスト用プリビルドスクリプト
 * フロントエンドとRustバイナリが未ビルドまたは古い場合にビルドする
 */
import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

const PROJECT_DIR = path.resolve(import.meta.dirname, '..', '..');
const FRONTEND_INDEX = path.join(PROJECT_DIR, 'build', 'index.html');
// 注: workspace 化により cargo build の出力先は <root>/target/ (旧: src-tauri/target/)
const TAURI_EXE = path.join(PROJECT_DIR, 'target', 'debug', 'liscov-tauri.exe');
const MOCK_SERVER_EXE = path.join(PROJECT_DIR, 'target', 'debug', 'mock-server.exe');

function needsBuild(artifact: string): boolean {
  return !fs.existsSync(artifact);
}

// フロントエンドビルド
if (needsBuild(FRONTEND_INDEX)) {
  console.log('[e2e-build] Building frontend...');
  execSync('pnpm build', { cwd: PROJECT_DIR, stdio: 'inherit' });
} else {
  console.log('[e2e-build] Frontend already built, skipping.');
}

// Rustバイナリビルド（Tauriアプリ + モックサーバー）
// 注: mock_server は別 workspace member (crates/mock-server) のため、
// --workspace で両方ビルドする必要がある (旧: --manifest-path src-tauri/Cargo.toml は src-tauri のみ)
if (needsBuild(TAURI_EXE) || needsBuild(MOCK_SERVER_EXE)) {
  console.log('[e2e-build] Building Rust binaries...');
  execSync('cargo build --workspace', { cwd: PROJECT_DIR, stdio: 'inherit' });
} else {
  console.log('[e2e-build] Rust binaries already built, skipping.');
}

console.log('[e2e-build] All artifacts ready.');
