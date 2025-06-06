use crate::gui::hooks::LiveChatHandle;
use dioxus::prelude::*;

/// ステータスパネルコンポーネント
/// Phase 4.2: 拡張された統計情報とビジュアル改善
#[component]
pub fn StatusPanel(live_chat_handle: LiveChatHandle) -> Element {
    // 統計情報を取得
    let stats = live_chat_handle.stats;
    let is_connected = live_chat_handle.is_connected;
    let state = live_chat_handle.state;
    let messages = live_chat_handle.messages;

    // 定期更新のためのシグナル
    let mut update_tick = use_signal(|| 0u32);

    // 1秒ごとに統計を更新
    use_effect(move || {
        if *is_connected.read() {
            spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    if !*is_connected.read() {
                        break;
                    }
                    let current_tick = *update_tick.read();
                    update_tick.set(current_tick + 1);
                }
            });
        }
    });

    // 計算された値
    let message_count = messages.read().len();
    let uptime = if *is_connected.read() {
        let seconds = stats.read().uptime_seconds;
        if seconds < 60 {
            format!("{}秒", seconds)
        } else if seconds < 3600 {
            format!("{}分{}秒", seconds / 60, seconds % 60)
        } else {
            format!("{}時間{}分", seconds / 3600, (seconds % 3600) / 60)
        }
    } else {
        "停止中".to_string()
    };

    // 接続状態のビジュアル
    let (status_icon, status_color, status_text) = match *state.read() {
        crate::gui::services::ServiceState::Connected => ("🟢", "#28a745", "接続中"),
        crate::gui::services::ServiceState::Connecting => ("🟡", "#ffc107", "接続中..."),
        crate::gui::services::ServiceState::Paused => ("⏸️", "#007bff", "一時停止"),
        crate::gui::services::ServiceState::Idle => ("⚪", "#6c757d", "待機中"),
        crate::gui::services::ServiceState::Error(_) => ("🔴", "#dc3545", "エラー"),
    };

    // メッセージ速度の判定
    let message_rate = stats.read().messages_per_minute;
    let (rate_status, rate_color) = if message_rate > 30.0 {
        ("🔥 活発", "#e53e3e")
    } else if message_rate > 10.0 {
        ("📈 普通", "#f6ad55")
    } else if message_rate > 0.0 {
        ("📊 静か", "#4299e1")
    } else {
        ("💤 休止", "#a0aec0")
    };

    rsx! {
        div {
            class: "status-panel",

            // ヘッダー
            div {
                class: "status-header",
                style: "
                    background: linear-gradient(135deg, #f7fafc 0%, #edf2f7 100%);
                    padding: 16px;
                    margin: -25px -25px 20px -25px;
                    border-bottom: 1px solid #e2e8f0;
                ",
                "📊 ライブ統計"
            }

            // 接続状態カード
            div {
                style: "
                    background: linear-gradient(135deg, #ffffff 0%, #f7fafc 100%);
                    border: 1px solid #e2e8f0;
                    border-radius: 12px;
                    padding: 16px;
                    margin-bottom: 20px;
                    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
                ",

                div {
                    style: "
                        display: flex;
                        justify-content: space-between;
                        align-items: center;
                        margin-bottom: 8px;
                    ",

                    div {
                        style: "display: flex; align-items: center; gap: 8px;",
                        span {
                            style: "font-size: 16px;",
                            "{status_icon}"
                        }
                        span {
                            style: format!("
                                font-weight: 600;
                                font-size: 14px;
                                color: {};
                            ", status_color),
                            "{status_text}"
                        }
                    }

                    div {
                        style: format!("
                            background: {};
                            color: white;
                            padding: 4px 8px;
                            border-radius: 12px;
                            font-size: 10px;
                            font-weight: 700;
                            text-transform: uppercase;
                            letter-spacing: 0.5px;
                        ", status_color),
                        if *is_connected.read() { "LIVE" } else { "OFFLINE" }
                    }
                }

                if let crate::gui::services::ServiceState::Error(ref error_msg) = *state.read() {
                    div {
                        style: "
                            background: #fed7d7;
                            color: #822727;
                            padding: 8px 12px;
                            border-radius: 6px;
                            font-size: 12px;
                            margin-top: 8px;
                        ",
                        "⚠️ {error_msg}"
                    }
                }
            }

            // 統計情報グリッド
            div {
                class: "stats-grid",
                style: "
                    display: grid;
                    grid-template-columns: 1fr 1fr;
                    gap: 16px;
                    margin-bottom: 20px;
                ",

                // メッセージ数
                div {
                    class: "stat-item",
                    style: "
                        background: linear-gradient(135deg, #eff6ff 0%, #dbeafe 100%);
                        border: 1px solid #bfdbfe;
                    ",
                    div {
                        class: "stat-value",
                        style: "color: #1e40af;",
                        "{message_count}"
                    }
                    div {
                        class: "stat-label",
                        "💬 メッセージ"
                    }
                }

                // 稼働時間
                div {
                    class: "stat-item",
                    style: "
                        background: linear-gradient(135deg, #f0fff4 0%, #dcfce7 100%);
                        border: 1px solid #bbf7d0;
                    ",
                    div {
                        class: "stat-value",
                        style: "color: #166534; font-size: 18px;",
                        "{uptime}"
                    }
                    div {
                        class: "stat-label",
                        "⏱️ 稼働時間"
                    }
                }

                // メッセージ速度
                div {
                    class: "stat-item",
                    style: "
                        background: linear-gradient(135deg, #fff7ed 0%, #fed7aa 100%);
                        border: 1px solid #fdba74;
                    ",
                    div {
                        class: "stat-value",
                        style: "color: #9a3412; font-size: 16px;",
                        "{stats.read().messages_per_minute:.1}/分"
                    }
                    div {
                        class: "stat-label",
                        "📈 速度"
                    }
                }

                // 活動状況
                div {
                    class: "stat-item",
                    style: "
                        background: linear-gradient(135deg, #fef2f2 0%, #fecaca 100%);
                        border: 1px solid #fca5a5;
                    ",
                    div {
                        class: "stat-value",
                        style: format!("color: {}; font-size: 14px;", rate_color),
                        "{rate_status}"
                    }
                    div {
                        class: "stat-label",
                        "🎯 活動度"
                    }
                }
            }

            // 詳細情報（アコーディオンスタイル）
            details {
                style: "
                    background: #f7fafc;
                    border: 1px solid #e2e8f0;
                    border-radius: 8px;
                    padding: 0;
                    margin-top: 16px;
                ",

                summary {
                    style: "
                        padding: 12px 16px;
                        cursor: pointer;
                        font-weight: 600;
                        color: #4a5568;
                        background: linear-gradient(135deg, #f7fafc 0%, #edf2f7 100%);
                        border-radius: 8px 8px 0 0;
                        transition: all 0.2s ease;
                    ",
                    "🔍 詳細統計"
                }

                div {
                    style: "padding: 16px;",

                    div {
                        style: "
                            display: grid;
                            grid-template-columns: 1fr 1fr;
                            gap: 12px;
                            font-size: 12px;
                        ",

                        div {
                            strong { "最終メッセージ:" }
                            br {}
                            span {
                                style: "color: #718096;",
                                {
                                    if let Some(last_time) = stats.read().last_message_time {
                                        last_time.format("%H:%M:%S").to_string()
                                    } else {
                                        "なし".to_string()
                                    }
                                }
                            }
                        }

                        div {
                            strong { "平均間隔:" }
                            br {}
                            span {
                                style: "color: #718096;",
                                {
                                    let message_rate = stats.read().messages_per_minute;
                                    if message_rate > 0.0 {
                                        format!("{:.1}秒", 60.0 / message_rate)
                                    } else {
                                        "計算中".to_string()
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // パフォーマンス指標
            div {
                style: "
                    margin-top: 16px;
                    padding: 12px;
                    background: linear-gradient(135deg, #f0f9ff 0%, #e0f2fe 100%);
                    border-radius: 8px;
                    border: 1px solid #bae6fd;
                ",

                div {
                    style: "
                        font-size: 11px;
                        color: #0369a1;
                        font-weight: 600;
                        margin-bottom: 4px;
                    ",
                    "⚡ パフォーマンス"
                }

                div {
                    style: "font-size: 10px; color: #075985;",
                    "Memory: Normal | CPU: Low | Network: Active"
                }
            }
        }
    }
}
