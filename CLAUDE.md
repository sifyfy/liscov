# CLAUDE.md

liscov-tauri プロジェクトの開発ガイド

## プロジェクト概要

**liscov-tauri** は YouTube Live Chat Monitor の Tauri + SvelteKit 版。元の liscov (Rust + Dioxus) から移行したバージョン。

### 技術スタック
- **Backend**: Tauri v2 + Rust
- **Frontend**: SvelteKit + Tailwind CSS + Svelte 5 Runes
- **Database**: SQLite (rusqlite)
- **TTS**: 棒読みちゃん / VOICEVOX 対応

## 開発コマンド

### フロントエンド

```bash
# 開発サーバー
pnpm dev

# ビルド
pnpm build

# 型チェック
pnpm check

# E2Eテスト
pnpm test:e2e
```

### Rust (バックエンド)

**重要**: Git Bash から直接 `cargo` を実行すると、Git の `link.exe` と Visual Studio の `link.exe` が競合してビルドが失敗する場合がある。

#### 解決方法: Developer PowerShell を使用

```powershell
# PowerShell から Visual Studio DevShell を起動してビルド
powershell -NoProfile -Command "& {
    Import-Module 'C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Microsoft.VisualStudio.DevShell.dll'
    Enter-VsDevShell -VsInstallPath 'C:\Program Files\Microsoft Visual Studio\2022\Community' -SkipAutomaticLocation
    Set-Location 'C:\Users\cat\dev\liscov-tauri'
    cargo check --manifest-path src-tauri/Cargo.toml
}"
```

または、バッチファイルを使用:

```batch
@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat" -no_logo
cd /d C:\Users\cat\dev\liscov-tauri
cargo build --manifest-path src-tauri/Cargo.toml
```

### Tauri

```bash
# 開発モード（フロントエンド + バックエンド同時起動）
pnpm tauri dev

# リリースビルド
pnpm tauri build
```

## プロジェクト構造

```
liscov-tauri/
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── commands/             # Tauri commands
│   │   ├── core/                 # コアモジュール
│   │   │   └── models/           # データモデル
│   │   └── services/             # バックグラウンドサービス
│   └── Cargo.toml
├── src/                          # SvelteKit Frontend
│   ├── lib/
│   │   ├── components/           # UIコンポーネント
│   │   ├── stores/               # Svelte stores
│   │   ├── tauri/                # Tauri API wrappers
│   │   └── types/                # TypeScript型定義
│   └── routes/
├── e2e/                          # E2Eテスト (Playwright)
└── package.json
```

## 元liscovとの対応

元の liscov の機能を移行する際は、以下のディレクトリを参照:

| 元liscov | liscov-tauri |
|----------|--------------|
| `src/gui/components/` | `src/lib/components/` |
| `src/gui/models.rs` | `src-tauri/src/commands/chat.rs` + `src/lib/types/` |
| `src/database/` | `src-tauri/src/core/` |
| `src/api/` | `src-tauri/src/services/` |

## 重要な注意事項

1. **Svelte 5 Runes**: `$state`, `$derived`, `$effect` を使用
2. **CSS変数**: `app.css` でカラーテーマを定義（`--primary-start`, `--bg-main` 等）
3. **Tauri Events**: フロントエンドへのリアルタイム通知に使用
