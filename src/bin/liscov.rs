use dioxus::prelude::*;
use liscov::{
    LiscovResult,
    gui::{components::MainWindow, config_manager, utils, plugin_system::PluginManager},
};
use std::sync::{Arc, Mutex};

/// ウィンドウ設定の保存用
static LAST_WINDOW_CONFIG: Mutex<Option<config_manager::WindowConfig>> = Mutex::new(None);

/// Dioxus 0.6.3ベースのliscov GUI アプリケーション
/// Slintから移行 (Phase 0-1: 技術検証・基本構造)
fn app() -> Element {
    let window = dioxus::desktop::use_window();

    // ウィンドウ状態を定期的に更新（軽量な監視）
    use_effect({
        let window = window.clone();
        move || {
            let window = window.clone();
            spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
                loop {
                    interval.tick().await;

                    // ウィンドウの現在状態を取得
                    let current_size = window.inner_size();
                    let current_position = window.outer_position().unwrap_or_default();
                    let is_maximized = window.is_maximized();

                    let window_config = config_manager::WindowConfig {
                        width: current_size.width,
                        height: current_size.height,
                        x: current_position.x,
                        y: current_position.y,
                        maximized: is_maximized,
                    };

                    // 最新の状態をグローバルに保存
                    if let Ok(mut last_config) = LAST_WINDOW_CONFIG.lock() {
                        *last_config = Some(window_config);
                    }
                }
            });
        }
    });

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

fn main() -> LiscovResult<()> {
    // tokio-consoleの初期化（プロファイリング用）
    #[cfg(feature = "debug-tokio")]
    console_subscriber::init();

    // 強化されたログ初期化
    #[cfg(not(feature = "debug-tokio"))]
    utils::init_logging()?;

    tracing::info!("🎬 Starting liscov GUI - YouTube Live Chat Monitor");
    tracing::debug!("📱 Starting Dioxus desktop application...");

    // 既存の設定管理システムを使用
    let config_manager = config_manager::ConfigManager::new()?;
    let mut config = config_manager.load_config().unwrap_or_else(|e| {
        tracing::warn!("設定読み込みエラー、デフォルト設定を使用: {}", e);
        config_manager::AppConfig::default()
    });

    // プラグインシステムを初期化
    let plugin_manager = Arc::new(PluginManager::new());
    tracing::info!("🔌 Plugin system initialized");

    // ウィンドウ位置をデスクトップ範囲内に調整
    utils::validate_window_bounds(&mut config.window);

    tracing::info!(
        "🪟 ウィンドウ設定: {}x{} at ({}, {}), 最大化: {}",
        config.window.width,
        config.window.height,
        config.window.x,
        config.window.y,
        config.window.maximized
    );

    // Dioxus 0.6.3のLaunchBuilderを使用してウィンドウ設定を適用
    let mut launch_builder = dioxus::LaunchBuilder::desktop();

    // ウィンドウ設定を適用
    launch_builder = launch_builder.with_cfg(
        dioxus::desktop::Config::new().with_window(
            dioxus::desktop::tao::window::WindowBuilder::new()
                .with_title("liscov - YouTube Live Chat Monitor")
                .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(
                    config.window.width as f64,
                    config.window.height as f64,
                ))
                .with_position(dioxus::desktop::tao::dpi::LogicalPosition::new(
                    config.window.x as f64,
                    config.window.y as f64,
                ))
                .with_maximized(config.window.maximized)
                .with_resizable(true),
        ),
    );

    // Ctrl+Cシグナルハンドラー
    ctrlc::set_handler(move || {
        tracing::info!("🛑 終了シグナルを受信しました");
        save_window_config_on_exit();
        std::process::exit(0);
    }).map_err(|e| liscov::GuiError::Configuration(format!("Failed to set signal handler: {}", e)))?;

    // Dioxusアプリケーションを起動
    launch_builder.launch(app);

    // 正常終了時の設定保存
    save_window_config_on_exit();

    tracing::info!("👋 liscov GUI shutting down");
    Ok(())
}

/// 終了時にウィンドウ設定を保存
fn save_window_config_on_exit() {
    if let Ok(last_config_guard) = LAST_WINDOW_CONFIG.lock() {
        if let Some(window_config) = last_config_guard.as_ref() {
            // 新しいConfigManagerインスタンスを作成
            if let Ok(config_manager) = config_manager::ConfigManager::new() {
                // 既存の設定を読み込み、ウィンドウ設定のみ更新
                if let Ok(mut config) = config_manager.load_config() {
                    config.window = window_config.clone();

                    if let Err(e) = config_manager.save_config(&config) {
                        tracing::error!("設定保存エラー: {}", e);
                    } else {
                        tracing::info!(
                            "💾 ウィンドウ設定を保存しました: {}x{} at ({}, {}), 最大化: {}",
                            config.window.width,
                            config.window.height,
                            config.window.x,
                            config.window.y,
                            config.window.maximized
                        );
                    }
                } else {
                    tracing::warn!("既存設定の読み込みに失敗しました");
                }
            } else {
                tracing::error!("ConfigManagerの作成に失敗しました");
            }
        } else {
            tracing::warn!("保存する最新のウィンドウ設定が見つかりませんでした");
        }
    }
}
