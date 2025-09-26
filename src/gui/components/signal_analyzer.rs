//! Signal分析パネル (Phase 4.1)
//!
//! Signal依存関係グラフの可視化と最適化推奨事項の表示

use crate::gui::performance_monitor::{
    check_performance_warnings, generate_performance_report, get_performance_stats,
};
use crate::gui::signal_optimizer::{
    generate_signal_analysis_report, get_batch_stats, get_optimization_recommendations,
    get_signal_graph, OptimizationType,
};

// Phase 4.3: クロージャ最適化統計の追加
use crate::gui::closure_optimizer::{generate_closure_optimization_report, get_closure_optimizer};

use dioxus::prelude::*;

/// Signal分析パネル
#[component]
pub fn SignalAnalyzer() -> Element {
    let mut show_analysis = use_signal(|| false);
    let mut analysis_report = use_signal(|| String::new());
    let mut last_update = use_signal(|| std::time::Instant::now());

    // 定期的にレポートを更新
    use_effect({
        let mut analysis_report = analysis_report.clone();
        let mut last_update = last_update.clone();

        move || {
            spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                    let report = generate_signal_analysis_report();
                    analysis_report.set(report);
                    last_update.set(std::time::Instant::now());
                }
            });
        }
    });

    rsx! {
        div {
            class: "signal-analyzer-panel",
            style: "
                padding: 16px;
                background: #f8fafc;
                border: 1px solid #e2e8f0;
                border-radius: 8px;
                margin: 8px;
            ",

            // ヘッダー
            div {
                style: "
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 16px;
                ",
                h3 {
                    style: "
                        margin: 0;
                        font-size: 16px;
                        font-weight: 600;
                        color: #374151;
                    ",
                    "📊 Signal Analysis (Phase 4.1)"
                }

                div {
                    style: "display: flex; gap: 8px; align-items: center;",

                    button {
                        class: "px-3 py-1 bg-blue-500 hover:bg-blue-600 text-white rounded text-sm",
                        onclick: move |_| {
                            let report = generate_signal_analysis_report();
                            analysis_report.set(report);
                            last_update.set(std::time::Instant::now());
                        },
                        "🔄 更新"
                    }

                    button {
                        class: if *show_analysis.read() {
                            "px-3 py-1 bg-gray-600 text-white rounded text-sm"
                        } else {
                            "px-3 py-1 bg-gray-500 hover:bg-gray-600 text-white rounded text-sm"
                        },
                        onclick: move |_| {
                            let current = *show_analysis.read();
                            show_analysis.set(!current);
                        },
                        if *show_analysis.read() { "📊 非表示" } else { "📊 表示" }
                    }
                }
            }

                                    // Phase 4.1 & 4.2 & 5.2: 統合統計パネル
            {
                let signal_stats = if let Ok(graph) = get_signal_graph().lock() {
                    let stats = graph.get_stats();
                    format!("Signals: {} | 重複: {} | 未使用: {}",
                           stats.total_signals,
                           stats.duplicate_signals,
                           stats.unused_signals)
                } else {
                    "Signal統計を取得できませんでした".to_string()
                };

                let batch_stats = if let Some(batch_stats) = get_batch_stats() {
                    format!(
                        "Batch: {} | 高優先度: {} | DOM: {} | 平均サイズ: {:.1}",
                        batch_stats.total_batched,
                        batch_stats.high_priority_count,
                        batch_stats.dom_update_count,
                        batch_stats.average_batch_size
                    )
                } else {
                    "Batch統計を取得できませんでした".to_string()
                };

                let perf_stats = if let Some(perf_stats) = get_performance_stats() {
                    format!(
                        "CPU: {:.1}% | メモリ: {:.1}MB | FPS: {:.1} | イベント: {}",
                        perf_stats.avg_cpu_usage,
                        perf_stats.avg_memory_usage as f64 / 1024.0 / 1024.0,
                        perf_stats.avg_fps,
                        perf_stats.total_events
                    )
                } else {
                    "パフォーマンス統計を取得できませんでした".to_string()
                };

                // Phase 4.3: クロージャ最適化統計
                let closure_stats = if let Ok(stats) = get_closure_optimizer().lock() {
                    format!(
                        "作成: {} | 再利用: {} | 節約: {:.1}KB | WeakRef: {} | クリーンアップ: {}",
                        stats.total_closures_created,
                        stats.closures_reused,
                        stats.memory_saved_bytes as f64 / 1024.0,
                        stats.weak_connections,
                        stats.cleanup_operations
                    )
                } else {
                    "クロージャ統計を取得できませんでした".to_string()
                };

                rsx! {
                    div {
                        style: "
                            background: white;
                            padding: 12px;
                            border-radius: 6px;
                            border: 1px solid #e5e7eb;
                            margin-bottom: 12px;
                            font-size: 13px;
                            color: #374151;
                        ",

                        // Phase 4.1: Signal統計
                        div {
                            style: "margin-bottom: 8px;",
                            span {
                                style: "font-weight: 600; color: #2563eb;",
                                "📊 Phase 4.1: "
                            }
                            "{signal_stats}"
                        }

                        // Phase 4.2: Batch統計
                        div {
                            style: "margin-bottom: 8px;",
                            span {
                                style: "font-weight: 600; color: #dc2626;",
                                "📦 Phase 4.2: "
                            }
                            "{batch_stats}"
                        }

                        // Phase 5.2: パフォーマンス統計
                        div {
                            style: "margin-bottom: 8px;",
                            span {
                                style: "font-weight: 600; color: #059669;",
                                "⚡ Phase 5.2: "
                            }
                            "{perf_stats}"
                        }

                        // Phase 4.3: クロージャ最適化統計
                        div {
                            style: "margin-bottom: 8px;",
                            span {
                                style: "font-weight: 600; color: #7c3aed;",
                                "🧹 Phase 4.3: "
                            }
                            "{closure_stats}"
                        }

                        // 最終更新時刻
                        div {
                            style: "
                                font-size: 12px;
                                color: #6b7280;
                                margin-top: 8px;
                                padding-top: 8px;
                                border-top: 1px solid #f3f4f6;
                            ",
                            "最終更新: {last_update.read().elapsed().as_secs()}秒前"
                        }
                    }
                }
            }

                        // Phase 5.2: パフォーマンス警告
            {
                let warnings = check_performance_warnings();

                if !warnings.is_empty() {
                    rsx! {
                        div {
                            style: "margin-bottom: 16px;",
                            h4 {
                                style: "
                                    margin: 0 0 8px 0;
                                    font-size: 14px;
                                    font-weight: 600;
                                    color: #f59e0b;
                                ",
                                "⚠️ パフォーマンス警告 ({warnings.len()})"
                            }

                            for (i, warning) in warnings.iter().enumerate() {
                                div {
                                    key: "{i}",
                                    style: "
                                        background: #fef3c7;
                                        color: #92400e;
                                        padding: 8px 12px;
                                        border-radius: 4px;
                                        border-left: 4px solid #f59e0b;
                                        margin-bottom: 6px;
                                        font-size: 13px;
                                        font-weight: 500;
                                    ",
                                    "{warning}"
                                }
                            }
                        }
                    }
                } else {
                    rsx! {
                        div {
                            style: "
                                background: #dcfce7;
                                color: #166534;
                                padding: 12px;
                                border-radius: 6px;
                                margin-bottom: 16px;
                                font-size: 14px;
                                font-weight: 500;
                            ",
                            "✅ パフォーマンスは正常です"
                        }
                    }
                }
            }

            // 最適化推奨事項
            {
                let recommendations = get_optimization_recommendations();

                if !recommendations.is_empty() {
                    rsx! {
                        div {
                            style: "margin-bottom: 16px;",
                            h4 {
                                style: "
                                    margin: 0 0 8px 0;
                                    font-size: 14px;
                                    font-weight: 600;
                                    color: #dc2626;
                                ",
                                "💡 最適化推奨事項 ({recommendations.len()})"
                            }

                            for (i, rec) in recommendations.iter().enumerate() {
                                div {
                                    key: "{i}",
                                    style: "
                                        background: white;
                                        padding: 10px;
                                        border-radius: 4px;
                                        border-left: 4px solid #f59e0b;
                                        margin-bottom: 8px;
                                        font-size: 13px;
                                    ",

                                    div {
                                        style: "
                                            display: flex;
                                            justify-content: space-between;
                                            align-items: center;
                                            margin-bottom: 4px;
                                        ",
                                        span {
                                            style: "font-weight: 600;",
                                            "{rec.description}"
                                        }
                                        span {
                                            style: "
                                                background: #fef3c7;
                                                color: #92400e;
                                                padding: 2px 6px;
                                                border-radius: 12px;
                                                font-size: 11px;
                                            ",
                                            "Priority {rec.priority}"
                                        }
                                    }

                                    div {
                                        style: "
                                            font-size: 12px;
                                            color: #6b7280;
                                        ",
                                        "期待改善: {rec.expected_improvement * 100.0:.1}% | 対象: {rec.signal_ids.len()} signals"
                                    }

                                    // 推奨事項の種類に応じたアクション
                                    match rec.recommendation_type {
                                        OptimizationType::MergeDuplicate => rsx! {
                                            div {
                                                style: "
                                                    margin-top: 6px;
                                                    font-size: 11px;
                                                    color: #059669;
                                                ",
                                                "🔄 重複したSignalを統合してメモリ使用量を削減できます"
                                            }
                                        },
                                        OptimizationType::RemoveUnused => rsx! {
                                            div {
                                                style: "
                                                    margin-top: 6px;
                                                    font-size: 11px;
                                                    color: #dc2626;
                                                ",
                                                "🗑️ 未使用のSignalを削除してパフォーマンスを向上できます"
                                            }
                                        },
                                        _ => rsx! {
                                            div {
                                                style: "
                                                    margin-top: 6px;
                                                    font-size: 11px;
                                                    color: #7c3aed;
                                                ",
                                                "⚡ Signal処理を最適化できます"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! {
                        div {
                            style: "
                                background: #dcfce7;
                                color: #166534;
                                padding: 12px;
                                border-radius: 6px;
                                margin-bottom: 16px;
                                font-size: 14px;
                            ",
                            "✅ 現在、最適化の必要はありません"
                        }
                    }
                }
            }

                        // 詳細分析レポート
            if *show_analysis.read() {
                div {
                    style: "
                        background: white;
                        border: 1px solid #e5e7eb;
                        border-radius: 6px;
                        overflow: hidden;
                        margin-bottom: 16px;
                    ",

                    div {
                        style: "
                            background: #f9fafb;
                            padding: 12px;
                            border-bottom: 1px solid #e5e7eb;
                            font-weight: 600;
                            font-size: 14px;
                            color: #374151;
                        ",
                        "📋 Signal分析レポート"
                    }

                    pre {
                        style: "
                            margin: 0;
                            padding: 16px;
                            font-family: 'Consolas', 'Monaco', monospace;
                            font-size: 12px;
                            line-height: 1.4;
                            color: #374151;
                            white-space: pre-wrap;
                            overflow-x: auto;
                        ",
                        "{analysis_report.read()}"
                    }
                }

                // Phase 5.2: パフォーマンスレポート
                div {
                    style: "
                        background: white;
                        border: 1px solid #e5e7eb;
                        border-radius: 6px;
                        overflow: hidden;
                        margin-bottom: 16px;
                    ",

                    div {
                        style: "
                            background: #f0f9ff;
                            padding: 12px;
                            border-bottom: 1px solid #e5e7eb;
                            font-weight: 600;
                            font-size: 14px;
                            color: #374151;
                            display: flex;
                            align-items: center;
                            gap: 8px;
                        ",
                        span {
                            style: "font-size: 16px;",
                            "⚡"
                        }
                        "パフォーマンスレポート (Phase 5.2)"
                    }

                    pre {
                        style: "
                            margin: 0;
                            padding: 16px;
                            font-family: 'Consolas', 'Monaco', monospace;
                            font-size: 12px;
                            line-height: 1.4;
                            color: #374151;
                            white-space: pre-wrap;
                            overflow-x: auto;
                            background: #fafafa;
                        ",
                        "{generate_performance_report()}"
                    }
                }

                // Phase 4.3: クロージャ最適化レポート
                div {
                    style: "
                        background: white;
                        border: 1px solid #e5e7eb;
                        border-radius: 6px;
                        overflow: hidden;
                    ",

                    div {
                        style: "
                            background: #faf5ff;
                            padding: 12px;
                            border-bottom: 1px solid #e5e7eb;
                            font-weight: 600;
                            font-size: 14px;
                            color: #374151;
                            display: flex;
                            align-items: center;
                            gap: 8px;
                        ",
                        span {
                            style: "font-size: 16px;",
                            "🧹"
                        }
                        "クロージャ最適化レポート (Phase 4.3)"
                    }

                    pre {
                        style: "
                            margin: 0;
                            padding: 16px;
                            font-family: 'Consolas', 'Monaco', monospace;
                            font-size: 12px;
                            line-height: 1.4;
                            color: #374151;
                            white-space: pre-wrap;
                            overflow-x: auto;
                            background: #fefcff;
                        ",
                        "{generate_closure_optimization_report()}"
                    }
                }
            }
        }
    }
}
