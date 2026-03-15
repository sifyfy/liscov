# ウィンドウ状態管理

## 目的（Why）

ユーザーが調整したウィンドウのサイズと位置を記憶し、次回起動時に同じ配置で復元する。配信中のマルチモニター環境で、毎回ウィンドウを配置し直す手間を省く。

## 振る舞い（What）

### 保存と復元

| イベント | 結果 |
|---------|------|
| アプリ終了時 | 現在のウィンドウサイズ・位置を `.window-state.json` に保存 |
| アプリ起動時（保存ファイルあり） | 保存されたサイズ・位置でウィンドウを復元 |
| アプリ起動時（保存ファイルなし/破損） | デフォルト値（1200x800）で表示 |
| 保存位置のモニターが存在しない | OSが適切な位置に配置 |
| 保存サイズが最小サイズ（800x600）未満 | 最小サイズを適用 |
| 保存サイズが画面より大きい | 画面サイズに自動調整 |

## 制約・不変条件（Boundaries）

| 制約 | 理由 |
|------|------|
| 最小ウィンドウサイズは800x600 | UIが崩れずに表示可能な最低限のサイズ |
| 最大化状態・フルスクリーン状態は保存しない（`StateFlags::SIZE \| StateFlags::POSITION` のみ） | 最大化状態の復元は環境によって挙動が不安定なため |
| ウィンドウ状態は `config.toml` とは別ファイル（`.window-state.json`）で管理する | tauri-plugin-window-stateが管理するファイルであり、アプリ設定とは独立 |

## 保存ファイル

### 保存先

| OS | パス |
|----|------|
| Windows | `%APPDATA%/com.liscov-tauri.app/.window-state.json` |
| macOS | `~/Library/Application Support/com.liscov-tauri.app/.window-state.json` |
| Linux | `~/.config/com.liscov-tauri.app/.window-state.json` |

> **Note**: パスは `tauri.conf.json` の `identifier` に基づく。

### ファイル形式

JSON形式で保存（tauri-plugin-window-stateによる自動管理）。

```json
{
  "main": {
    "x": 100,
    "y": 100,
    "width": 1200,
    "height": 800
  }
}
```

## 保存項目

| 項目 | 型 | 説明 |
|-----|-----|------|
| `x` | i32 | ウィンドウのX座標 |
| `y` | i32 | ウィンドウのY座標 |
| `width` | u32 | ウィンドウ幅 |
| `height` | u32 | ウィンドウ高さ |

## 動作フロー

### アプリ起動時（復元）

```
1. .window-state.jsonの存在確認
   ├─ 存在する → ファイルを読み込み
   └─ 存在しない → デフォルト値を使用（1200x800）
        ↓
2. 保存位置の検証
   ├─ モニター範囲内 → 保存位置・サイズを適用
   └─ モニター範囲外 → OSが適切な位置に配置
        ↓
3. サイズの検証
   ├─ 最小サイズ未満 → 最小サイズ（800x600）を適用
   ├─ 画面より大きい → 画面サイズに調整
   └─ 範囲内 → 保存サイズを適用
        ↓
4. ウィンドウを表示
```

### アプリ終了時（保存）

```
1. ユーザーがウィンドウを閉じる
        ↓
2. tauri-plugin-window-stateが現在のサイズ・位置を取得
        ↓
3. .window-state.jsonに書き込み
        ↓
4. アプリ終了
```

## 境界条件

| 状況 | 動作 |
|------|------|
| 初回起動 | tauri.conf.jsonのデフォルト値（1200x800） |
| 保存ファイル不正/破損 | デフォルト値を使用 |
| 保存位置のモニターが存在しない | OSが適切な位置に配置 |
| 保存位置が画面外 | OSが適切な位置に配置 |
| 保存サイズが最小サイズ（800x600）未満 | 最小サイズを適用 |
| 保存サイズが画面より大きい | 画面サイズに自動調整 |

## 実装方法

### 使用プラグイン

`tauri-plugin-window-state`（Tauri v2公式プラグイン）を使用。

### 設定

```rust
// src-tauri/src/lib.rs
use tauri_plugin_window_state::{StateFlags, WindowExt};

tauri::Builder::default()
    .plugin(
        tauri_plugin_window_state::Builder::default()
            .with_state_flags(StateFlags::SIZE | StateFlags::POSITION)
            .build()
    )
```

### 保存フラグ

`StateFlags::SIZE | StateFlags::POSITION` のみを指定。最大化状態やフルスクリーン状態は保存しない。

## 既存設定との関係

ウィンドウ状態は `config.toml` とは別ファイルで管理される。

| 設定 | ファイル | 用途 |
|------|---------|------|
| アプリ設定 | `config.toml` | フォントサイズ、TTS設定など |
| ウィンドウ状態 | `.window-state.json` | ウィンドウサイズ・位置 |

## E2Eテスト

`e2e/window-state.spec.ts` でテスト。

### テストケース

1. **サイズ保存・復元**: リサイズ → 終了 → 再起動 → サイズが復元される
2. **位置保存・復元**: 移動 → 終了 → 再起動 → 位置が復元される

### テストデータ分離

テスト実行時は `.window-state.json` を削除してクリーンな状態で開始。
