use anyhow::Result;
use dioxus::prelude::*;
use liscov::gui::{components::MainWindow, utils};

/// Dioxus 0.6.3ベースのliscov GUI アプリケーション
/// Slintから移行 (Phase 0-1: 技術検証・基本構造)
fn app() -> Element {
    rsx! {
        div {
            class: "app",
            style: "
                height: 100vh;
                margin: 0;
                padding: 0;
                overflow: hidden;
                background: #f0f2f5;
                font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            ",

            MainWindow {}
        }
    }
}

fn main() -> Result<()> {
    // tokio-consoleの初期化（プロファイリング用）
    #[cfg(feature = "debug-tokio")]
    console_subscriber::init();

    // 強化されたログ初期化
    #[cfg(not(feature = "debug-tokio"))]
    utils::init_logging()?;

    tracing::info!("🎬 Starting liscov GUI - YouTube Live Chat Monitor");
    tracing::debug!("📱 Starting Dioxus desktop application...");

    // Dioxus 0.6.3の正しいAPIでアプリケーションを起動
    // 内部でtokioランタイムが管理されるため、外部でtokio::mainは不要
    dioxus::launch(app);

    tracing::info!("👋 liscov GUI shutting down");
    Ok(())
}
