//! TTS設定コンポーネント
//!
//! 棒読みちゃん/VOICEVOX連携の設定UI

use dioxus::prelude::*;

use crate::gui::plugins::tts_plugin::backends::TtsBackend;
use crate::gui::plugins::tts_plugin::config::{TtsBackendType, TtsConfig};
use crate::gui::plugins::tts_plugin::launcher;
use crate::gui::tts_manager::get_tts_manager;

/// TTS設定コンポーネント
#[component]
pub fn TtsSettings() -> Element {
    // 設定状態
    let mut config = use_signal(TtsConfig::default);
    let mut connection_status = use_signal(|| ConnectionStatus::Unknown);
    let mut is_testing = use_signal(|| false);

    // 保存済み設定を読み込み、TTSマネージャーを初期化
    use_effect(move || {
        spawn(async move {
            if let Ok(config_manager) =
                crate::gui::unified_config::UnifiedConfigManager::new().await
            {
                if let Ok(Some(saved_config)) = config_manager
                    .get_typed_config::<TtsConfig>("tts_config")
                    .await
                {
                    // TTSマネージャーを更新
                    {
                        let tts_manager = get_tts_manager();
                        let mut mgr = tts_manager.write().await;
                        mgr.update_config(saved_config.clone()).await;
                    }

                    config.set(saved_config);
                    tracing::debug!("🔊 TTS設定を読み込みました");
                }
            }
        });
    });

    // 設定を保存
    let save_config = move |new_config: TtsConfig| {
        spawn(async move {
            // 永続化
            if let Ok(config_manager) =
                crate::gui::unified_config::UnifiedConfigManager::new().await
            {
                let _ = config_manager
                    .set_typed_config("tts_config", &new_config)
                    .await;
                let _ = config_manager.flush_dirty_configs().await;
                tracing::info!("🔊 TTS設定を保存しました");
            }

            // TTSマネージャーを更新
            {
                let tts_manager = get_tts_manager();
                let mut mgr = tts_manager.write().await;
                mgr.update_config(new_config.clone()).await;
            }

            config.set(new_config);
        });
    };

    // 接続テスト
    let test_connection = move |_| {
        let current_config = config.read().clone();
        spawn(async move {
            is_testing.set(true);
            connection_status.set(ConnectionStatus::Testing);

            let result = match current_config.backend {
                TtsBackendType::None => {
                    connection_status.set(ConnectionStatus::Unknown);
                    is_testing.set(false);
                    return;
                }
                TtsBackendType::Bouyomichan => {
                    use crate::gui::plugins::tts_plugin::backends::BouyomichanBackend;
                    let backend = BouyomichanBackend::new(current_config.bouyomichan.clone());
                    backend.test_connection().await
                }
                TtsBackendType::Voicevox => {
                    use crate::gui::plugins::tts_plugin::backends::VoicevoxBackend;
                    let backend = VoicevoxBackend::new(current_config.voicevox.clone());
                    backend.test_connection().await
                }
            };

            match result {
                Ok(true) => connection_status.set(ConnectionStatus::Connected),
                Ok(false) => connection_status.set(ConnectionStatus::Failed("接続失敗".to_string())),
                Err(e) => connection_status.set(ConnectionStatus::Failed(e.to_string())),
            }
            is_testing.set(false);
        });
    };

    let current_config = config.read().clone();

    rsx! {
        div {
            style: "
                background: #f8f9fa;
                border: 1px solid #e9ecef;
                border-radius: 8px;
                padding: 16px;
                margin-bottom: 20px;
            ",

            h3 {
                style: "
                    margin: 0 0 16px 0;
                    color: #495057;
                ",
                "🔊 TTS読み上げ設定"
            }

            // 有効/無効切り替え
            div {
                style: "margin-bottom: 16px;",
                label {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 8px;
                        font-weight: 500;
                        color: #2d3748;
                        cursor: pointer;
                        font-size: 14px;
                    ",
                    input {
                        r#type: "checkbox",
                        checked: current_config.enabled,
                        style: "width: 16px; height: 16px; accent-color: #0d6efd;",
                        onchange: {
                            let save_config = save_config.clone();
                            move |evt| {
                                let mut new_config = config.read().clone();
                                new_config.enabled = evt.checked();
                                save_config(new_config);
                            }
                        }
                    }
                    "TTS読み上げを有効化"
                }
            }

            // バックエンド選択
            div {
                style: "margin-bottom: 16px;",

                label {
                    style: "
                        display: block;
                        font-weight: 500;
                        color: #2d3748;
                        margin-bottom: 8px;
                        font-size: 14px;
                    ",
                    "🎙️ バックエンド選択"
                }

                div {
                    style: "display: flex; gap: 16px;",

                    label {
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 6px;
                            cursor: pointer;
                        ",
                        input {
                            r#type: "radio",
                            name: "tts_backend",
                            checked: current_config.backend == TtsBackendType::Bouyomichan,
                            onchange: {
                                let save_config = save_config.clone();
                                move |_| {
                                    let mut new_config = config.read().clone();
                                    new_config.backend = TtsBackendType::Bouyomichan;
                                    save_config(new_config);
                                    connection_status.set(ConnectionStatus::Unknown);
                                }
                            }
                        }
                        "棒読みちゃん"
                    }

                    label {
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 6px;
                            cursor: pointer;
                        ",
                        input {
                            r#type: "radio",
                            name: "tts_backend",
                            checked: current_config.backend == TtsBackendType::Voicevox,
                            onchange: {
                                let save_config = save_config.clone();
                                move |_| {
                                    let mut new_config = config.read().clone();
                                    new_config.backend = TtsBackendType::Voicevox;
                                    save_config(new_config);
                                    connection_status.set(ConnectionStatus::Unknown);
                                }
                            }
                        }
                        "VOICEVOX"
                    }

                    label {
                        style: "
                            display: flex;
                            align-items: center;
                            gap: 6px;
                            cursor: pointer;
                        ",
                        input {
                            r#type: "radio",
                            name: "tts_backend",
                            checked: current_config.backend == TtsBackendType::None,
                            onchange: {
                                let save_config = save_config.clone();
                                move |_| {
                                    let mut new_config = config.read().clone();
                                    new_config.backend = TtsBackendType::None;
                                    save_config(new_config);
                                    connection_status.set(ConnectionStatus::Unknown);
                                }
                            }
                        }
                        "なし"
                    }
                }
            }

            // バックエンド固有設定
            if current_config.backend == TtsBackendType::Bouyomichan {
                BouyomichanSettings {
                    config: config,
                    on_save: save_config.clone()
                }
            }

            if current_config.backend == TtsBackendType::Voicevox {
                VoicevoxSettings {
                    config: config,
                    on_save: save_config.clone()
                }
            }

            // 読み上げオプション
            if current_config.backend != TtsBackendType::None {
                ReadingOptions {
                    config: config,
                    on_save: save_config.clone()
                }
            }

            // 接続テストボタン
            if current_config.backend != TtsBackendType::None {
                {
                    let opacity = if *is_testing.read() { "0.6" } else { "1" };
                    let button_style = format!(
                        "padding: 8px 16px; \
                         background: linear-gradient(135deg, #28a745 0%, #218838 100%); \
                         color: white; \
                         border: none; \
                         border-radius: 6px; \
                         cursor: pointer; \
                         font-size: 14px; \
                         font-weight: 500; \
                         opacity: {};",
                        opacity
                    );
                    rsx! {
                        div {
                            style: "
                                margin-top: 16px;
                                padding-top: 16px;
                                border-top: 1px solid #dee2e6;
                                display: flex;
                                align-items: center;
                                gap: 12px;
                            ",

                            button {
                                style: "{button_style}",
                                disabled: *is_testing.read(),
                                onclick: test_connection,
                                if *is_testing.read() {
                                    "テスト中..."
                                } else {
                                    "🔗 接続テスト"
                                }
                            }

                            // 接続ステータス表示
                            ConnectionStatusBadge { status: connection_status }
                        }
                    }
                }
            }

            // 説明文
            div {
                style: "
                    background: #e8f4fd;
                    border: 1px solid #b8daff;
                    border-radius: 4px;
                    padding: 12px;
                    margin-top: 16px;
                ",
                p {
                    style: "margin: 0 0 8px 0; font-weight: bold; color: #0056b3;",
                    "💡 TTS読み上げについて"
                }
                ul {
                    style: "margin: 0; padding-left: 20px; font-size: 13px;",
                    li { "棒読みちゃん: 事前に棒読みちゃんを起動してください" }
                    li { "VOICEVOX: 事前にVOICEVOXエンジンを起動してください" }
                    li { "接続テストで動作確認ができます" }
                }
            }
        }
    }
}

/// 接続ステータス
#[derive(Clone, PartialEq)]
enum ConnectionStatus {
    Unknown,
    Testing,
    Connected,
    Failed(String),
}

/// 接続ステータスバッジ
#[component]
fn ConnectionStatusBadge(status: Signal<ConnectionStatus>) -> Element {
    let status_value = status.read().clone();

    let (text, bg_color, text_color) = match &status_value {
        ConnectionStatus::Unknown => ("未テスト", "#6c757d", "white"),
        ConnectionStatus::Testing => ("テスト中...", "#ffc107", "#212529"),
        ConnectionStatus::Connected => ("接続成功", "#28a745", "white"),
        ConnectionStatus::Failed(msg) => {
            let display_msg = if msg.len() > 20 {
                format!("失敗: {}...", &msg[..20])
            } else {
                format!("失敗: {}", msg)
            };
            return rsx! {
                span {
                    style: "
                        padding: 4px 12px;
                        border-radius: 12px;
                        font-size: 12px;
                        font-weight: 500;
                        background: #dc3545;
                        color: white;
                    ",
                    title: "{msg}",
                    "{display_msg}"
                }
            };
        }
    };

    rsx! {
        span {
            style: "
                padding: 4px 12px;
                border-radius: 12px;
                font-size: 12px;
                font-weight: 500;
                background: {bg_color};
                color: {text_color};
            ",
            "{text}"
        }
    }
}

/// 棒読みちゃん設定
#[component]
fn BouyomichanSettings(
    config: Signal<TtsConfig>,
    on_save: EventHandler<TtsConfig>,
) -> Element {
    let current = config.read().bouyomichan.clone();

    rsx! {
        div {
            style: "
                background: #fff;
                border: 1px solid #dee2e6;
                border-radius: 6px;
                padding: 12px;
                margin-bottom: 16px;
            ",

            h4 {
                style: "margin: 0 0 12px 0; color: #495057; font-size: 14px;",
                "棒読みちゃん設定"
            }

            // ホストとポート
            div {
                style: "display: flex; gap: 12px; margin-bottom: 12px;",

                div {
                    style: "flex: 2;",
                    label {
                        style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                        "ホスト"
                    }
                    input {
                        r#type: "text",
                        value: "{current.host}",
                        style: "
                            width: 100%;
                            padding: 6px 10px;
                            border: 1px solid #ced4da;
                            border-radius: 4px;
                            font-size: 13px;
                            box-sizing: border-box;
                        ",
                        onchange: move |evt| {
                            let mut new_config = config.read().clone();
                            new_config.bouyomichan.host = evt.value();
                            on_save.call(new_config);
                        }
                    }
                }

                div {
                    style: "flex: 1;",
                    label {
                        style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                        "ポート"
                    }
                    input {
                        r#type: "number",
                        value: "{current.port}",
                        style: "
                            width: 100%;
                            padding: 6px 10px;
                            border: 1px solid #ced4da;
                            border-radius: 4px;
                            font-size: 13px;
                            box-sizing: border-box;
                        ",
                        onchange: move |evt| {
                            if let Ok(port) = evt.value().parse::<u16>() {
                                let mut new_config = config.read().clone();
                                new_config.bouyomichan.port = port;
                                on_save.call(new_config);
                            }
                        }
                    }
                }
            }

            // 音声パラメータ
            div {
                style: "display: flex; gap: 12px;",

                div {
                    style: "flex: 1;",
                    label {
                        style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                        "音量 (-1=デフォルト)"
                    }
                    input {
                        r#type: "number",
                        min: "-1",
                        max: "100",
                        value: "{current.volume}",
                        style: "
                            width: 100%;
                            padding: 6px 10px;
                            border: 1px solid #ced4da;
                            border-radius: 4px;
                            font-size: 13px;
                            box-sizing: border-box;
                        ",
                        onchange: move |evt| {
                            if let Ok(volume) = evt.value().parse::<i32>() {
                                let mut new_config = config.read().clone();
                                new_config.bouyomichan.volume = volume;
                                on_save.call(new_config);
                            }
                        }
                    }
                }

                div {
                    style: "flex: 1;",
                    label {
                        style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                        "速度 (-1=デフォルト)"
                    }
                    input {
                        r#type: "number",
                        min: "-1",
                        max: "300",
                        value: "{current.speed}",
                        style: "
                            width: 100%;
                            padding: 6px 10px;
                            border: 1px solid #ced4da;
                            border-radius: 4px;
                            font-size: 13px;
                            box-sizing: border-box;
                        ",
                        onchange: move |evt| {
                            if let Ok(speed) = evt.value().parse::<i32>() {
                                let mut new_config = config.read().clone();
                                new_config.bouyomichan.speed = speed;
                                on_save.call(new_config);
                            }
                        }
                    }
                }

                div {
                    style: "flex: 1;",
                    label {
                        style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                        "音程 (-1=デフォルト)"
                    }
                    input {
                        r#type: "number",
                        min: "-1",
                        max: "300",
                        value: "{current.tone}",
                        style: "
                            width: 100%;
                            padding: 6px 10px;
                            border: 1px solid #ced4da;
                            border-radius: 4px;
                            font-size: 13px;
                            box-sizing: border-box;
                        ",
                        onchange: move |evt| {
                            if let Ok(tone) = evt.value().parse::<i32>() {
                                let mut new_config = config.read().clone();
                                new_config.bouyomichan.tone = tone;
                                on_save.call(new_config);
                            }
                        }
                    }
                }
            }

            // 自動起動設定
            AutoLaunchSettings {
                backend: TtsBackendType::Bouyomichan,
                auto_launch: current.auto_launch,
                auto_close_on_exit: current.auto_close_on_exit,
                executable_path: current.executable_path.clone(),
                on_auto_launch_change: move |enabled| {
                    let mut new_config = config.read().clone();
                    new_config.bouyomichan.auto_launch = enabled;
                    on_save.call(new_config);
                },
                on_auto_close_change: move |enabled| {
                    let mut new_config = config.read().clone();
                    new_config.bouyomichan.auto_close_on_exit = enabled;
                    on_save.call(new_config);
                },
                on_path_change: move |path: Option<String>| {
                    let mut new_config = config.read().clone();
                    new_config.bouyomichan.executable_path = path;
                    on_save.call(new_config);
                }
            }
        }
    }
}

/// VOICEVOX設定
#[component]
fn VoicevoxSettings(
    config: Signal<TtsConfig>,
    on_save: EventHandler<TtsConfig>,
) -> Element {
    let current = config.read().voicevox.clone();

    rsx! {
        div {
            style: "
                background: #fff;
                border: 1px solid #dee2e6;
                border-radius: 6px;
                padding: 12px;
                margin-bottom: 16px;
            ",

            h4 {
                style: "margin: 0 0 12px 0; color: #495057; font-size: 14px;",
                "VOICEVOX設定"
            }

            div {
                style: "display: flex; gap: 12px;",

                div {
                    style: "flex: 2;",
                    label {
                        style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                        "ホスト"
                    }
                    input {
                        r#type: "text",
                        value: "{current.host}",
                        style: "
                            width: 100%;
                            padding: 6px 10px;
                            border: 1px solid #ced4da;
                            border-radius: 4px;
                            font-size: 13px;
                            box-sizing: border-box;
                        ",
                        onchange: move |evt| {
                            let mut new_config = config.read().clone();
                            new_config.voicevox.host = evt.value();
                            on_save.call(new_config);
                        }
                    }
                }

                div {
                    style: "flex: 1;",
                    label {
                        style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                        "ポート"
                    }
                    input {
                        r#type: "number",
                        value: "{current.port}",
                        style: "
                            width: 100%;
                            padding: 6px 10px;
                            border: 1px solid #ced4da;
                            border-radius: 4px;
                            font-size: 13px;
                            box-sizing: border-box;
                        ",
                        onchange: move |evt| {
                            if let Ok(port) = evt.value().parse::<u16>() {
                                let mut new_config = config.read().clone();
                                new_config.voicevox.port = port;
                                on_save.call(new_config);
                            }
                        }
                    }
                }

                div {
                    style: "flex: 1;",
                    label {
                        style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                        "話者ID"
                    }
                    input {
                        r#type: "number",
                        min: "0",
                        value: "{current.speaker_id}",
                        style: "
                            width: 100%;
                            padding: 6px 10px;
                            border: 1px solid #ced4da;
                            border-radius: 4px;
                            font-size: 13px;
                            box-sizing: border-box;
                        ",
                        onchange: move |evt| {
                            if let Ok(speaker_id) = evt.value().parse::<i32>() {
                                let mut new_config = config.read().clone();
                                new_config.voicevox.speaker_id = speaker_id;
                                on_save.call(new_config);
                            }
                        }
                    }
                }
            }

            // 話者ID説明
            p {
                style: "margin: 8px 0 0 0; font-size: 11px; color: #6c757d;",
                "話者ID: 0=四国めたん, 1=ずんだもん, 2=四国めたん(あまあま), 3=ずんだもん(あまあま)..."
            }

            // 音声パラメータスライダー
            {
                let volume_percent = (current.volume_scale * 100.0) as i32;
                let speed_percent = (current.speed_scale * 100.0) as i32;
                let pitch_value = (current.pitch_scale * 100.0) as i32; // -15〜15
                let intonation_percent = (current.intonation_scale * 100.0) as i32;

                rsx! {
                    div {
                        style: "margin-top: 12px; display: flex; flex-direction: column; gap: 12px;",

                        // 音量
                        div {
                            label {
                                style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                                "音量: {volume_percent}%"
                            }
                            div {
                                style: "display: flex; align-items: center; gap: 8px;",
                                span { style: "font-size: 11px; color: #999; width: 35px;", "0%" }
                                input {
                                    r#type: "range",
                                    min: "0",
                                    max: "200",
                                    value: "{volume_percent}",
                                    style: "flex: 1;",
                                    oninput: move |evt| {
                                        if let Ok(v) = evt.value().parse::<f32>() {
                                            let mut new_config = config.read().clone();
                                            new_config.voicevox.volume_scale = v / 100.0;
                                            on_save.call(new_config);
                                        }
                                    }
                                }
                                span { style: "font-size: 11px; color: #999; width: 35px;", "200%" }
                            }
                        }

                        // 話速
                        div {
                            label {
                                style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                                "話速: {speed_percent}%"
                            }
                            div {
                                style: "display: flex; align-items: center; gap: 8px;",
                                span { style: "font-size: 11px; color: #999; width: 35px;", "50%" }
                                input {
                                    r#type: "range",
                                    min: "50",
                                    max: "200",
                                    value: "{speed_percent}",
                                    style: "flex: 1;",
                                    oninput: move |evt| {
                                        if let Ok(v) = evt.value().parse::<f32>() {
                                            let mut new_config = config.read().clone();
                                            new_config.voicevox.speed_scale = v / 100.0;
                                            on_save.call(new_config);
                                        }
                                    }
                                }
                                span { style: "font-size: 11px; color: #999; width: 35px;", "200%" }
                            }
                        }

                        // 音高
                        div {
                            label {
                                style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                                "音高: {pitch_value}"
                            }
                            div {
                                style: "display: flex; align-items: center; gap: 8px;",
                                span { style: "font-size: 11px; color: #999; width: 35px;", "-15" }
                                input {
                                    r#type: "range",
                                    min: "-15",
                                    max: "15",
                                    value: "{pitch_value}",
                                    style: "flex: 1;",
                                    oninput: move |evt| {
                                        if let Ok(v) = evt.value().parse::<f32>() {
                                            let mut new_config = config.read().clone();
                                            new_config.voicevox.pitch_scale = v / 100.0;
                                            on_save.call(new_config);
                                        }
                                    }
                                }
                                span { style: "font-size: 11px; color: #999; width: 35px;", "+15" }
                            }
                        }

                        // 抑揚
                        div {
                            label {
                                style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                                "抑揚: {intonation_percent}%"
                            }
                            div {
                                style: "display: flex; align-items: center; gap: 8px;",
                                span { style: "font-size: 11px; color: #999; width: 35px;", "0%" }
                                input {
                                    r#type: "range",
                                    min: "0",
                                    max: "200",
                                    value: "{intonation_percent}",
                                    style: "flex: 1;",
                                    oninput: move |evt| {
                                        if let Ok(v) = evt.value().parse::<f32>() {
                                            let mut new_config = config.read().clone();
                                            new_config.voicevox.intonation_scale = v / 100.0;
                                            on_save.call(new_config);
                                        }
                                    }
                                }
                                span { style: "font-size: 11px; color: #999; width: 35px;", "200%" }
                            }
                        }
                    }
                }
            }

            // 自動起動設定
            AutoLaunchSettings {
                backend: TtsBackendType::Voicevox,
                auto_launch: current.auto_launch,
                auto_close_on_exit: current.auto_close_on_exit,
                executable_path: current.executable_path.clone(),
                on_auto_launch_change: move |enabled| {
                    let mut new_config = config.read().clone();
                    new_config.voicevox.auto_launch = enabled;
                    on_save.call(new_config);
                },
                on_auto_close_change: move |enabled| {
                    let mut new_config = config.read().clone();
                    new_config.voicevox.auto_close_on_exit = enabled;
                    on_save.call(new_config);
                },
                on_path_change: move |path: Option<String>| {
                    let mut new_config = config.read().clone();
                    new_config.voicevox.executable_path = path;
                    on_save.call(new_config);
                }
            }
        }
    }
}

/// 読み上げオプション
#[component]
fn ReadingOptions(
    config: Signal<TtsConfig>,
    on_save: EventHandler<TtsConfig>,
) -> Element {
    let current = config.read().clone();

    rsx! {
        div {
            style: "
                background: #fff;
                border: 1px solid #dee2e6;
                border-radius: 6px;
                padding: 12px;
                margin-bottom: 16px;
            ",

            h4 {
                style: "margin: 0 0 12px 0; color: #495057; font-size: 14px;",
                "読み上げオプション"
            }

            div {
                style: "display: flex; flex-direction: column; gap: 8px;",

                label {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 8px;
                        cursor: pointer;
                        font-size: 13px;
                    ",
                    input {
                        r#type: "checkbox",
                        checked: current.read_author_name,
                        style: "width: 14px; height: 14px;",
                        onchange: move |evt| {
                            let mut new_config = config.read().clone();
                            new_config.read_author_name = evt.checked();
                            on_save.call(new_config);
                        }
                    }
                    "投稿者名を読み上げる"
                }

                // 投稿者名のサブオプション（投稿者名読み上げ有効時のみ表示）
                if current.read_author_name {
                    div {
                        style: "margin-left: 24px; display: flex; flex-direction: column; gap: 6px;",

                        label {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 8px;
                                cursor: pointer;
                                font-size: 12px;
                                color: #495057;
                            ",
                            input {
                                r#type: "checkbox",
                                checked: current.add_honorific,
                                style: "width: 14px; height: 14px;",
                                onchange: move |evt| {
                                    let mut new_config = config.read().clone();
                                    new_config.add_honorific = evt.checked();
                                    on_save.call(new_config);
                                }
                            }
                            "敬称「さん」を付ける"
                        }

                        label {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 8px;
                                cursor: pointer;
                                font-size: 12px;
                                color: #495057;
                            ",
                            input {
                                r#type: "checkbox",
                                checked: current.strip_at_prefix,
                                style: "width: 14px; height: 14px;",
                                onchange: move |evt| {
                                    let mut new_config = config.read().clone();
                                    new_config.strip_at_prefix = evt.checked();
                                    on_save.call(new_config);
                                }
                            }
                            "@で始まる場合は@を除去する"
                        }

                        label {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 8px;
                                cursor: pointer;
                                font-size: 12px;
                                color: #495057;
                            ",
                            input {
                                r#type: "checkbox",
                                checked: current.strip_handle_suffix,
                                style: "width: 14px; height: 14px;",
                                onchange: move |evt| {
                                    let mut new_config = config.read().clone();
                                    new_config.strip_handle_suffix = evt.checked();
                                    on_save.call(new_config);
                                }
                            }
                            "末尾の-xxx(ハンドルsuffix)を除去する"
                        }

                        p {
                            style: "margin: 4px 0 0 0; font-size: 11px; color: #6c757d;",
                            "※読み仮名が設定されている場合は上記の除去処理は適用されません"
                        }
                    }
                }

                label {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 8px;
                        cursor: pointer;
                        font-size: 13px;
                    ",
                    input {
                        r#type: "checkbox",
                        checked: current.read_superchat_amount,
                        style: "width: 14px; height: 14px;",
                        onchange: move |evt| {
                            let mut new_config = config.read().clone();
                            new_config.read_superchat_amount = evt.checked();
                            on_save.call(new_config);
                        }
                    }
                    "スーパーチャット金額を読み上げる"
                }
            }

            // 最大文字数
            div {
                style: "margin-top: 12px;",

                label {
                    style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                    "最大読み上げ文字数: {current.max_text_length}文字"
                }

                input {
                    r#type: "range",
                    min: "50",
                    max: "500",
                    value: "{current.max_text_length}",
                    style: "width: 100%;",
                    oninput: move |evt| {
                        if let Ok(len) = evt.value().parse::<usize>() {
                            let mut new_config = config.read().clone();
                            new_config.max_text_length = len;
                            on_save.call(new_config);
                        }
                    }
                }
            }
        }
    }
}

/// 自動起動設定コンポーネント
#[component]
fn AutoLaunchSettings(
    backend: TtsBackendType,
    auto_launch: bool,
    auto_close_on_exit: bool,
    executable_path: Option<String>,
    on_auto_launch_change: EventHandler<bool>,
    on_auto_close_change: EventHandler<bool>,
    on_path_change: EventHandler<Option<String>>,
) -> Element {
    let mut launch_status = use_signal(|| LaunchStatus::Idle);
    let mut detected_path = use_signal(|| None::<String>);

    // 初回レンダリング時にパスを自動検出
    {
        let backend_clone = backend.clone();
        let executable_path_clone = executable_path.clone();
        use_effect(move || {
            if executable_path_clone.is_none() {
                if let Some(path) = launcher::detect_executable(backend_clone.clone()) {
                    detected_path.set(Some(path));
                }
            }
        });
    }

    // 表示用のパス（設定値 → 検出値 → 空）
    let display_path = executable_path
        .clone()
        .or_else(|| detected_path.read().clone())
        .unwrap_or_default();

    let backend_name = match &backend {
        TtsBackendType::Bouyomichan => "棒読みちゃん",
        TtsBackendType::Voicevox => "VOICEVOX",
        TtsBackendType::None => "",
    };

    // 起動ボタンハンドラ
    let backend_for_launch = backend.clone();
    let executable_path_for_launch = executable_path.clone();
    let handle_launch = move |_| {
        let path = executable_path_for_launch
            .clone()
            .or_else(|| detected_path.read().clone());

        launch_status.set(LaunchStatus::Launching);

        match launcher::launch_backend(backend_for_launch.clone(), path.as_deref()) {
            Ok(()) => {
                launch_status.set(LaunchStatus::Success);
            }
            Err(e) => {
                launch_status.set(LaunchStatus::Error(e));
            }
        }
    };

    rsx! {
        div {
            style: "
                margin-top: 12px;
                padding-top: 12px;
                border-top: 1px solid #e9ecef;
            ",

            // 自動起動チェックボックス
            label {
                style: "
                    display: flex;
                    align-items: center;
                    gap: 8px;
                    cursor: pointer;
                    font-size: 13px;
                    margin-bottom: 8px;
                ",
                input {
                    r#type: "checkbox",
                    checked: auto_launch,
                    style: "width: 14px; height: 14px;",
                    onchange: move |evt| {
                        on_auto_launch_change.call(evt.checked());
                    }
                }
                "自動起動を有効化"
            }

            // アプリ終了時に一緒に終了するチェックボックス
            label {
                style: "
                    display: flex;
                    align-items: center;
                    gap: 8px;
                    cursor: pointer;
                    font-size: 13px;
                    margin-bottom: 8px;
                ",
                input {
                    r#type: "checkbox",
                    checked: auto_close_on_exit,
                    style: "width: 14px; height: 14px;",
                    onchange: move |evt| {
                        on_auto_close_change.call(evt.checked());
                    }
                }
                "アプリ終了時に一緒に終了する"
            }

            // 実行ファイルパス
            div {
                style: "margin-bottom: 8px;",
                label {
                    style: "display: block; font-size: 12px; color: #6c757d; margin-bottom: 4px;",
                    "実行ファイル"
                }
                div {
                    style: "display: flex; gap: 4px;",
                    input {
                        r#type: "text",
                        value: "{display_path}",
                        placeholder: "自動検出または手動で入力",
                        style: "
                            flex: 1;
                            padding: 6px 10px;
                            border: 1px solid #ced4da;
                            border-radius: 4px;
                            font-size: 12px;
                            box-sizing: border-box;
                        ",
                        onchange: move |evt| {
                            let value = evt.value();
                            if value.is_empty() {
                                on_path_change.call(None);
                            } else {
                                on_path_change.call(Some(value));
                            }
                        }
                    }
                    button {
                        style: "
                            padding: 6px 12px;
                            background: #f8f9fa;
                            border: 1px solid #ced4da;
                            border-radius: 4px;
                            cursor: pointer;
                            font-size: 12px;
                            white-space: nowrap;
                        ",
                        onclick: move |_| {
                            spawn(async move {
                                let file = rfd::AsyncFileDialog::new()
                                    .add_filter("実行ファイル", &["exe"])
                                    .pick_file()
                                    .await;

                                if let Some(file) = file {
                                    let path = file.path().to_string_lossy().to_string();
                                    on_path_change.call(Some(path));
                                }
                            });
                        },
                        "参照..."
                    }
                }
            }

            // 起動ボタンとステータス
            div {
                style: "display: flex; align-items: center; gap: 8px;",

                button {
                    style: "
                        padding: 6px 12px;
                        background: linear-gradient(135deg, #6c757d 0%, #5a6268 100%);
                        color: white;
                        border: none;
                        border-radius: 4px;
                        cursor: pointer;
                        font-size: 12px;
                    ",
                    onclick: handle_launch,
                    "🚀 {backend_name}を起動"
                }

                // ステータス表示
                match &*launch_status.read() {
                    LaunchStatus::Idle => rsx! {},
                    LaunchStatus::Launching => rsx! {
                        span { style: "font-size: 12px; color: #6c757d;", "起動中..." }
                    },
                    LaunchStatus::Success => rsx! {
                        span { style: "font-size: 12px; color: #28a745;", "✓ 起動しました" }
                    },
                    LaunchStatus::Error(e) => rsx! {
                        span { style: "font-size: 12px; color: #dc3545;", "✗ {e}" }
                    },
                }
            }
        }
    }
}

/// 起動ステータス
#[derive(Clone, PartialEq)]
enum LaunchStatus {
    Idle,
    Launching,
    Success,
    Error(String),
}
