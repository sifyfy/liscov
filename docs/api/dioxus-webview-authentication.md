# Dioxus Desktop WebView認証フロー調査結果

このドキュメントは、liscovにおけるメンバー限定配信対応のためのWebView認証フロー実装について調査した結果をまとめたものです。

## 概要

### 採用する認証方式

**WebViewログイン方式**を採用します。

| 比較項目 | DB直接抽出 | WebViewログイン |
|----------|-----------|----------------|
| UX | ボタン一発 | アプリ内で完結 |
| 透明性 | △ 暗黙的 | ◎ 明示的 |
| 実装 | △ OS毎に異なる | ◎ 統一 |
| 保守性 | △ ブラウザ依存 | ◎ 独立 |

### 技術スタック確認

| コンポーネント | バージョン | Cookie API |
|---------------|-----------|------------|
| Dioxus | 0.7 | - |
| dioxus-desktop | 0.7.2 | - |
| Wry | 0.53.5 | ◎ 対応 |

## 実装可能性調査結果

### 1. WebViewからのCookie取得

**結論: 実装可能**

#### Wry 0.53.5 Cookie API

```rust
use wry::WebView;

// すべてのCookieを取得
let all_cookies = webview.cookies()?;

// 特定ドメインのCookieのみ取得（推奨）
let youtube_cookies = webview.cookies_for_url("https://www.youtube.com")?;

// Cookieを設定
webview.set_cookie(&cookie)?;

// Cookieを削除
webview.delete_cookie(&cookie)?;
```

#### Dioxus DesktopからWebViewへのアクセス

`DesktopService`構造体の公開フィールドを使用:

```rust
use dioxus::prelude::*;

fn component() -> Element {
    let desktop = use_window();

    // DesktopService.webview にアクセス
    // desktop.webview.cookies_for_url("https://www.youtube.com")

    rsx! { /* ... */ }
}
```

**注意**: `DesktopService.webview`フィールドは`pub`で公開されており、直接アクセス可能です。

### 2. ログイン完了検知

**結論: Cookieポーリングで実装可能**

#### 問題点

Dioxus DesktopのConfigからWryの`with_navigation_handler`を直接設定する公式APIがありません。

#### 代替方法: Cookieポーリング

YouTubeログイン完了は`SAPISID` Cookieの存在で判断できます:

```rust
async fn check_login_status(webview: &WebView) -> bool {
    match webview.cookies_for_url("https://www.youtube.com") {
        Ok(cookies) => {
            cookies.iter().any(|c| c.name() == "SAPISID")
        }
        Err(_) => false,
    }
}

// ポーリング例
loop {
    if check_login_status(&webview).await {
        // ログイン完了
        break;
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

### 3. 認証用ウィンドウの作成

**結論: 複数の方法あり**

#### 方法A: Dioxusの新規ウィンドウ機能（未検証）

Dioxus 0.7では複数ウィンドウのサポートがありますが、Windows環境での安定性に懸念があります（Issue #2483）。

#### 方法B: Wry/Taoで直接作成

Dioxusとは別にWryで認証専用WebViewを作成:

```rust
use wry::{WebViewBuilder, WebContext};
use tao::{
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn create_auth_window() -> Result<WebView> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("YouTube ログイン")
        .with_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)?;

    let webview = WebViewBuilder::new()
        .with_url("https://accounts.google.com/")
        .build(&window)?;

    Ok(webview)
}
```

## 推奨実装アーキテクチャ

### 認証フロー

```
[メンバー限定配信を有効化ボタン]
    ↓
認証状態を確認（既存Cookieチェック）
    ↓ 未認証
認証ウィンドウを開く
    ↓
YouTubeにナビゲート（youtube.com）
    ↓
ユーザーがGoogleログイン
    ↓
Cookieポーリング（1秒間隔）
    ↓ SAPISID検出
必要なCookie（SID, HSID, SSID, APISID, SAPISID）を抽出
    ↓
アプリ内に保存
    ↓
認証ウィンドウを閉じる
    ↓
InnerTube APIリクエストに認証ヘッダー追加
```

### モジュール構成案

```
src/
├── api/
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── cookie_manager.rs    # Cookie保存・読み込み
│   │   ├── sapisidhash.rs       # SAPISIDHASH生成
│   │   └── webview_auth.rs      # WebView認証フロー
│   └── innertube.rs             # 認証ヘッダー追加
└── gui/
    └── components/
        └── auth_window.rs       # 認証UIコンポーネント
```

### Cookie保存形式

```toml
# ~/.config/liscov/credentials.toml
[youtube]
sid = "..."
hsid = "..."
ssid = "..."
apisid = "..."
sapisid = "..."
expires_at = "2026-01-01T00:00:00Z"
```

## 制限事項と注意点

### プラットフォーム制限

| OS | Cookie取得 | 備考 |
|----|-----------|------|
| Windows | ◎ | WebView2使用 |
| macOS | ◎ | WebKit使用 |
| Linux | ◎ | WebKitGTK使用 |
| Android | ✗ | 未サポート |

### セキュリティ考慮事項

1. **Cookie保存**: 平文保存は避け、OS資格情報マネージャーの利用を検討
2. **ログ出力**: Cookie値をログに出力しない
3. **有効期限管理**: 期限切れ時の再認証フロー

### 既知の問題

1. **Windows複数ウィンドウ**: Dioxusで複数ウィンドウ使用時に不安定な場合あり（Issue #2483）
2. **HttpOnly Cookie**: 一部Cookieはセキュリティ制限でアクセス不可の可能性

## 次のステップ

1. [ ] 認証モジュール（`src/api/auth/`）の基本構造作成
2. [ ] SAPISIDHASH生成ロジック実装
3. [ ] WebView認証フローのプロトタイプ実装
4. [ ] 認証UIコンポーネント作成
5. [ ] InnerTube APIへの認証ヘッダー統合

## 参考資料

- [Dioxus Desktop Guide](https://dioxuslabs.com/learn/0.6/guides/desktop/)
- [Wry WebView API](https://docs.rs/wry/0.53.5/wry/struct.WebView.html)
- [Dioxus Desktop API](https://docs.rs/dioxus-desktop/0.7.2/dioxus_desktop/)
- [Cookie取得API PR](https://github.com/tauri-apps/wry/pull/1394)
- [Dioxus複数ウィンドウIssue](https://github.com/DioxusLabs/dioxus/issues/2483)
