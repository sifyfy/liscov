/**
 * E2Eテスト用プリビルドスクリプト
 * フロントエンドとRustバイナリが未ビルドまたは古い場合にビルドする
 */
import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

const PROJECT_DIR = path.resolve(import.meta.dirname, '..', '..');
const FRONTEND_INDEX = path.join(PROJECT_DIR, 'build', 'index.html');
const TAURI_EXE = path.join(PROJECT_DIR, 'src-tauri', 'target', 'debug', 'liscov-tauri.exe');
const MOCK_SERVER_EXE = path.join(PROJECT_DIR, 'src-tauri', 'target', 'debug', 'mock_server.exe');

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
const cargoPath = path.join(PROJECT_DIR, 'src-tauri', 'Cargo.toml');
if (needsBuild(TAURI_EXE) || needsBuild(MOCK_SERVER_EXE)) {
  console.log('[e2e-build] Building Rust binaries...');
  execSync(`cargo build --manifest-path ${cargoPath}`, { cwd: PROJECT_DIR, stdio: 'inherit' });
} else {
  console.log('[e2e-build] Rust binaries already built, skipping.');
}

console.log('[e2e-build] All artifacts ready.');
