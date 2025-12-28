use clap::Parser;
use dioxus::prelude::*;
use liscov::{
    gui::{components::MainWindow, config_manager, plugin_system::PluginManager, utils},
    LiscovResult,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// ウィンドウ設定の保存用
static LAST_WINDOW_CONFIG: Mutex<Option<config_manager::WindowConfig>> = Mutex::new(None);

/// CLI引数の定義
#[derive(Parser, Debug)]
#[command(name = "liscov")]
#[command(about = "YouTube Live Chat Monitor - ライブチャット監視ツール")]
#[command(version)]
struct Args {
    /// ログ出力ディレクトリを指定
    #[arg(long, value_name = "DIR")]
    log_dir: Option<PathBuf>,

    /// ログレベルを指定 (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    log_level: String,

    /// ファイルログ出力を無効化
    #[arg(long)]
    no_file_logging: bool,

    /// 保存するログファイル数の上限
    #[arg(long, value_name = "NUM", default_value = "30")]
    max_log_files: u32,
}

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
    // CLI引数を解析
    let args = Args::parse();

    // 環境変数でログディレクトリを取得（CLI引数より優先度低い）
    let env_log_dir = std::env::var("LISCOV_LOG_DIR").ok().map(PathBuf::from);

    // tokio-consoleの初期化（プロファイリング用）
    #[cfg(feature = "debug-tokio")]
    console_subscriber::init();

    // 既存の設定管理システムを使用してログ設定を取得
    let config_manager = config_manager::ConfigManager::new()?;
    let mut config = config_manager.load_config().unwrap_or_else(|e| {
        tracing::warn!("設定読み込みエラー、デフォルト設定を使用: {}", e);
        config_manager::AppConfig::default()
    });

    // CLI引数でログ設定を上書き
    if args.no_file_logging {
        config.log.enable_file_logging = false;
    }
    if !args.log_level.is_empty() {
        config.log.log_level = args.log_level;
    }
    config.log.max_log_files = args.max_log_files;

    // ログディレクトリ決定（優先度: CLI > 環境変数 > 設定ファイル > XDGデフォルト）
    let custom_log_dir = args.log_dir.or(env_log_dir);

    // 強化されたログ初期化
    #[cfg(not(feature = "debug-tokio"))]
    utils::init_logging_with_config(&config.log, custom_log_dir.clone())?;

    tracing::info!("🎬 Starting liscov GUI - YouTube Live Chat Monitor");
    tracing::debug!("📱 Starting Dioxus desktop application...");

    // ログ設定を表示
    if config.log.enable_file_logging {
        tracing::info!(
            "📁 ログ設定: ディレクトリ={:?}, レベル={}, 最大ファイル数={}",
            custom_log_dir.or(config.log.log_dir.clone()),
            config.log.log_level,
            config.log.max_log_files
        );
    } else {
        tracing::info!("📁 ファイルログ出力は無効化されています");
    }

    // プラグインシステムを初期化
    let _plugin_manager = Arc::new(PluginManager::new());
    tracing::info!("🔌 Plugin system initialized");

    // WebSocket APIサーバーを起動
    let ws_server = liscov::api::websocket_server::get_websocket_server();
    let ws_port = ws_server.port();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            if let Err(e) = ws_server.start().await {
                tracing::error!("❌ Failed to start WebSocket server: {}", e);
                return;
            }
            // サーバーが停止するまで待機（shutdownシグナルを待つ）
            loop {
                if !ws_server.is_running().await {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    });
    tracing::info!("🌐 WebSocket API server started on ws://127.0.0.1:{}", ws_port);

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
    })
    .map_err(|e| liscov::GuiError::Configuration(format!("Failed to set signal handler: {}", e)))?;

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
