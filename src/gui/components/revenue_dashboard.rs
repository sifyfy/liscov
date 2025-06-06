use crate::analytics::{RealtimeStats, RevenueAnalytics, RevenueSummary};
use dioxus::prelude::*;
use dioxus_charts::LineChart;

/// 収益ダッシュボードコンポーネント
#[component]
pub fn RevenueDashboard(analytics: Signal<RevenueAnalytics>) -> Element {
    // パフォーマンス最適化：計算結果をメモ化
    let summary = use_memo(move || analytics.read().get_summary());
    let realtime_stats = use_memo(move || analytics.read().get_realtime_stats());

    rsx! {
        div {
            class: "revenue-dashboard",
            style: "
                padding: 20px; 
                background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%); 
                border-radius: 12px; 
                margin: 10px;
                min-height: 100vh;
                box-sizing: border-box;
            ",

            h2 {
                style: "
                    color: #2c3e50; 
                    margin-bottom: 30px; 
                    text-align: center;
                    font-size: clamp(1.5rem, 4vw, 2.5rem);
                    font-weight: 700;
                    text-shadow: 0 2px 4px rgba(0,0,0,0.1);
                ",
                "💰 収益分析ダッシュボード"
            }

            // 収益サマリーカード
            RevenueSummaryCard { summary: summary() }

            // リアルタイム統計
            RealtimeStatsCard { stats: realtime_stats() }

            // グリッドレイアウト（レスポンシブ対応）
            div {
                style: "
                    display: grid;
                    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
                    gap: 20px;
                    margin-top: 20px;
                ",

                // 貢献者ランキング
                TopContributorsCard { analytics }

                // 時間別収益グラフ（dioxus-charts使用）
                HourlyRevenueCard { analytics }
            }
        }
    }
}

/// 収益サマリーカード
#[component]
fn RevenueSummaryCard(summary: RevenueSummary) -> Element {
    rsx! {
        div {
            class: "revenue-summary-card",
            style: "
                background: white; 
                padding: 25px; 
                border-radius: 12px; 
                margin-bottom: 25px; 
                box-shadow: 0 4px 12px rgba(0,0,0,0.1);
                border: 1px solid #e1e8ed;
                transition: transform 0.2s ease, box-shadow 0.2s ease;
            ",

            h3 {
                style: "
                    color: #2c3e50; 
                    margin-bottom: 20px;
                    font-size: 1.4rem;
                    font-weight: 600;
                    display: flex;
                    align-items: center;
                    gap: 10px;
                ",
                span { style: "font-size: 1.6rem;", "📊" }
                "収益サマリー"
            }

            div {
                style: "
                    display: grid; 
                    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); 
                    gap: 20px;
                ",

                StatCard {
                    title: "総収益",
                    value: format!("¥{:.0}", summary.total_revenue),
                    icon: "💰",
                    color: "#27ae60"
                }

                StatCard {
                    title: "Super Chat数",
                    value: summary.super_chat_count.to_string(),
                    icon: "💬",
                    color: "#3498db"
                }

                StatCard {
                    title: "平均金額",
                    value: format!("¥{:.0}", summary.average_super_chat),
                    icon: "📈",
                    color: "#e74c3c"
                }

                StatCard {
                    title: "新規メンバー",
                    value: summary.membership_gains.to_string(),
                    icon: "👥",
                    color: "#9b59b6"
                }
            }
        }
    }
}

/// リアルタイム統計カード
#[component]
fn RealtimeStatsCard(stats: RealtimeStats) -> Element {
    rsx! {
        div {
            class: "realtime-stats-card",
            style: "
                background: white; 
                padding: 25px; 
                border-radius: 12px; 
                margin-bottom: 25px; 
                box-shadow: 0 4px 12px rgba(0,0,0,0.1);
                border: 1px solid #e1e8ed;
            ",

            h3 {
                style: "
                    color: #2c3e50; 
                    margin-bottom: 20px;
                    font-size: 1.4rem;
                    font-weight: 600;
                    display: flex;
                    align-items: center;
                    gap: 10px;
                ",
                span {
                    style: "font-size: 1.6rem;",
                    "⚡"
                }
                "リアルタイム統計"
            }

            div {
                style: "
                    display: grid; 
                    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); 
                    gap: 20px;
                    margin-bottom: 25px;
                ",

                StatCard {
                    title: "分あたり収益",
                    value: format!("¥{:.0}/分", stats.revenue_per_minute),
                    icon: "⏱️",
                    color: "#f39c12"
                }

                StatCard {
                    title: "分あたりメッセージ",
                    value: format!("{}/分", stats.messages_per_minute),
                    icon: "💬",
                    color: "#1abc9c"
                }
            }

            // 金額分布
            AmountDistributionChart { distribution: stats.amount_distribution }
        }
    }
}

/// 統計カード（改良版）
#[component]
fn StatCard(title: String, value: String, icon: String, color: String) -> Element {
    rsx! {
        div {
            style: "
                background: linear-gradient(145deg, white 0%, #f8f9fa 100%); 
                padding: 20px; 
                border-radius: 8px; 
                border-left: 4px solid {color}; 
                box-shadow: 0 2px 8px rgba(0,0,0,0.08);
                transition: all 0.3s ease;
                cursor: pointer;
                position: relative;
                overflow: hidden;
            ",

            div {
                style: "
                    display: flex; 
                    align-items: center; 
                    justify-content: space-between;
                    position: relative;
                    z-index: 1;
                ",

                div {
                    h4 {
                        style: "
                            margin: 0; 
                            color: #7f8c8d; 
                            font-size: 0.9rem;
                            font-weight: 500;
                            text-transform: uppercase;
                            letter-spacing: 0.5px;
                        ",
                        "{title}"
                    }
                    p {
                        style: "
                            margin: 8px 0 0 0; 
                            font-size: clamp(1.3rem, 3vw, 1.8rem); 
                            font-weight: 700; 
                            color: {color};
                            line-height: 1.2;
                        ",
                        "{value}"
                    }
                }

                span {
                    style: "
                        font-size: clamp(2rem, 4vw, 2.5rem);
                        filter: drop-shadow(0 2px 4px rgba(0,0,0,0.1));
                        transition: transform 0.3s ease;
                    ",
                    "{icon}"
                }
            }
        }
    }
}

/// 金額分布チャート
#[component]
fn AmountDistributionChart(distribution: crate::analytics::AmountDistribution) -> Element {
    let total = distribution.under_100
        + distribution.range_100_500
        + distribution.range_500_1000
        + distribution.range_1000_5000
        + distribution.over_5000;

    rsx! {
        div {
            style: "margin-top: 20px;",

            h4 {
                style: "color: #2c3e50; margin-bottom: 15px; font-size: 1.1rem; font-weight: 600;",
                "💸 Super Chat金額分布"
            }

            if total > 0 {
                div {
                    style: "display: grid; gap: 10px;",

                    DistributionBar {
                        label: "¥100未満",
                        count: distribution.under_100,
                        total,
                        color: "#95a5a6"
                    }

                    DistributionBar {
                        label: "¥100-500",
                        count: distribution.range_100_500,
                        total,
                        color: "#3498db"
                    }

                    DistributionBar {
                        label: "¥500-1000",
                        count: distribution.range_500_1000,
                        total,
                        color: "#f39c12"
                    }

                    DistributionBar {
                        label: "¥1000-5000",
                        count: distribution.range_1000_5000,
                        total,
                        color: "#e67e22"
                    }

                    DistributionBar {
                        label: "¥5000以上",
                        count: distribution.over_5000,
                        total,
                        color: "#e74c3c"
                    }
                }
            } else {
                p {
                    style: "color: #7f8c8d; text-align: center; padding: 20px;",
                    "まだSuper Chatがありません"
                }
            }
        }
    }
}

/// 分布バー
#[component]
fn DistributionBar(label: String, count: usize, total: usize, color: String) -> Element {
    let percentage = if total > 0 {
        (count as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    rsx! {
        div {
            style: "display: flex; align-items: center; gap: 12px;",

            span {
                style: "min-width: 90px; font-size: 0.85rem; color: #7f8c8d; font-weight: 500;",
                "{label}"
            }

            div {
                style: "
                    flex: 1; 
                    background: #ecf0f1; 
                    border-radius: 12px; 
                    height: 24px; 
                    position: relative;
                    overflow: hidden;
                ",

                div {
                    style: "
                        background: linear-gradient(90deg, {color} 0%, {color}aa 100%); 
                        height: 100%; 
                        border-radius: 12px; 
                        width: {percentage}%; 
                        transition: width 0.5s ease;
                    ",
                }

                span {
                    style: "
                        position: absolute; 
                        right: 8px; 
                        top: 50%; 
                        transform: translateY(-50%); 
                        font-size: 0.8rem; 
                        color: #2c3e50; 
                        font-weight: 600;
                    ",
                    "{count}"
                }
            }
        }
    }
}

/// 上位貢献者カード
#[component]
fn TopContributorsCard(analytics: Signal<RevenueAnalytics>) -> Element {
    let contributors = &analytics.read().top_contributors;

    rsx! {
        div {
            class: "top-contributors-card",
            style: "
                background: white; 
                padding: 25px; 
                border-radius: 12px; 
                margin-bottom: 20px; 
                box-shadow: 0 4px 12px rgba(0,0,0,0.1);
                border: 1px solid #e1e8ed;
            ",

            h3 {
                style: "
                    color: #2c3e50; 
                    margin-bottom: 20px;
                    font-size: 1.4rem;
                    font-weight: 600;
                    display: flex;
                    align-items: center;
                    gap: 10px;
                ",
                span { style: "font-size: 1.6rem;", "🏆" }
                "上位貢献者"
            }

            if contributors.is_empty() {
                p {
                    style: "color: #7f8c8d; text-align: center; padding: 30px; font-style: italic;",
                    "まだ貢献者がいません"
                }
            } else {
                div {
                    style: "display: grid; gap: 12px;",

                    for (index, contributor) in contributors.iter().enumerate() {
                        ContributorRow {
                            rank: index + 1,
                            contributor: contributor.clone()
                        }
                    }
                }
            }
        }
    }
}

/// 貢献者行
#[component]
fn ContributorRow(rank: usize, contributor: crate::analytics::ContributorInfo) -> Element {
    let rank_icon = match rank {
        1 => "🥇",
        2 => "🥈",
        3 => "🥉",
        _ => "🏅",
    };

    rsx! {
        div {
            style: "
                display: flex; 
                align-items: center; 
                padding: 15px; 
                background: linear-gradient(145deg, #f8f9fa 0%, #ffffff 100%); 
                border-radius: 8px; 
                border-left: 3px solid #3498db;
                transition: transform 0.2s ease, box-shadow 0.2s ease;
                cursor: pointer;
            ",

            span {
                style: "font-size: 24px; margin-right: 15px;",
                "{rank_icon}"
            }

            div {
                style: "flex: 1;",

                h5 {
                    style: "margin: 0; color: #2c3e50; font-size: 1rem; font-weight: 600;",
                    "{contributor.display_name}"
                }

                p {
                    style: "margin: 4px 0 0 0; font-size: 0.8rem; color: #7f8c8d;",
                    "{contributor.contribution_count}回の貢献"
                }
            }

            span {
                style: "
                    font-weight: 700; 
                    color: #27ae60; 
                    font-size: 1.1rem;
                    background: #d5f4e6;
                    padding: 6px 12px;
                    border-radius: 6px;
                ",
                "¥{contributor.total_contribution:.0}"
            }
        }
    }
}

/// 時間別収益カード（dioxus-charts使用）
#[component]
fn HourlyRevenueCard(analytics: Signal<RevenueAnalytics>) -> Element {
    let hourly_data = &analytics.read().hourly_revenue;

    rsx! {
        div {
            style: "
                background: white; 
                padding: 25px; 
                border-radius: 12px; 
                margin-bottom: 20px; 
                box-shadow: 0 4px 12px rgba(0,0,0,0.1);
                border: 1px solid #e1e8ed;
            ",

            // ヘッダー部分（エクスポートボタン付き）
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;",

                h3 {
                    style: "
                        color: #2c3e50; 
                        margin: 0;
                        font-size: 1.4rem;
                        font-weight: 600;
                        display: flex;
                        align-items: center;
                        gap: 10px;
                    ",
                    span { style: "font-size: 1.6rem;", "📈" }
                    "時間別収益推移"
                }

                if !hourly_data.is_empty() {
                    button {
                        onclick: move |_| {
                            // エクスポート機能（プレースホルダー）
                            web_sys::window()
                                .unwrap()
                                .alert_with_message("CSV エクスポート機能は Week 17-18 で実装予定です")
                                .unwrap();
                        },
                        style: "
                            background: linear-gradient(135deg, #27ae60 0%, #2ecc71 100%);
                            color: white;
                            border: none;
                            padding: 8px 16px;
                            border-radius: 6px;
                            cursor: pointer;
                            font-size: 0.8rem;
                            font-weight: 600;
                            transition: all 0.3s ease;
                            box-shadow: 0 2px 6px rgba(39, 174, 96, 0.3);
                        ",
                        "📊 CSV出力"
                    }
                }
            }

            if hourly_data.is_empty() {
                p {
                    style: "color: #7f8c8d; text-align: center; padding: 40px; font-style: italic;",
                    "時間別データがまだありません"
                }
            } else if hourly_data.len() >= 2 {
                // dioxus-chartsのLineChartを使用
                div {
                    style: "height: 300px; margin: 20px 0;",

                    LineChart {
                        padding_top: 30,
                        padding_left: 70,
                        padding_right: 50,
                        padding_bottom: 50,
                        series: vec![
                            hourly_data
                                .iter()
                                .map(|h| h.super_chat_amount as f32)
                                .collect::<Vec<f32>>()
                        ],
                        labels: hourly_data
                            .iter()
                            .map(|h| h.hour.format("%H:00").to_string())
                            .collect::<Vec<String>>(),
                        label_interpolation: (|v| format!("¥{:.0}", v)) as fn(f32) -> String,
                    }
                }

                // 統計サマリー
                div {
                    style: "
                        display: grid; 
                        grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); 
                        gap: 15px; 
                        margin-top: 20px;
                        padding: 15px;
                        background: #f8f9fa;
                        border-radius: 8px;
                    ",

                    div {
                        style: "text-align: center;",
                        p {
                            style: "margin: 0; font-size: 0.8rem; color: #7f8c8d;",
                            "データ点数"
                        }
                        p {
                            style: "margin: 5px 0 0 0; font-size: 1.2rem; font-weight: 600; color: #2c3e50;",
                            "{hourly_data.len()}"
                        }
                    }

                    div {
                        style: "text-align: center;",
                        p {
                            style: "margin: 0; font-size: 0.8rem; color: #7f8c8d;",
                            "最高収益"
                        }
                        p {
                            style: "margin: 5px 0 0 0; font-size: 1.2rem; font-weight: 600; color: #27ae60;",
                            "¥{hourly_data.iter().map(|h| h.super_chat_amount).fold(0.0, f64::max):.0}"
                        }
                    }

                    div {
                        style: "text-align: center;",
                        p {
                            style: "margin: 0; font-size: 0.8rem; color: #7f8c8d;",
                            "平均収益"
                        }
                        p {
                            style: "margin: 5px 0 0 0; font-size: 1.2rem; font-weight: 600; color: #3498db;",
                            "¥{hourly_data.iter().map(|h| h.super_chat_amount).sum::<f64>() / hourly_data.len() as f64:.0}"
                        }
                    }
                }
            } else {
                div {
                    style: "text-align: center; padding: 20px;",
                    p {
                        style: "color: #2c3e50; font-size: 1.2rem; margin-bottom: 10px;",
                        "時間別データ件数: {hourly_data.len()}"
                    }
                    p {
                        style: "color: #7f8c8d; font-size: 0.9rem;",
                        "グラフ表示には2つ以上のデータ点が必要です"
                    }
                }
            }
        }
    }
}
