//! クロージャ最適化ユーティリティ (Phase 4.3)
//!
//! 不要なクロージャの削減と循環参照回避
//!
//! 注意: この版は簡略版です。RcのSend/Sync問題を回避するため、
//! ローカル最適化のみ提供し、グローバル状態は使用しません。

use std::sync::{Arc, Mutex, OnceLock};

/// クロージャ最適化統計 (簡略版)
#[derive(Debug, Clone)]
pub struct ClosureOptimizationStats {
    pub total_closures_created: u64,
    pub closures_reused: u64,
    pub memory_saved_bytes: u64,
    pub weak_connections: u64,
    pub cleanup_operations: u64,
}

impl ClosureOptimizationStats {
    /// 新しい統計を作成
    pub fn new() -> Self {
        Self {
            total_closures_created: 0,
            closures_reused: 0,
            memory_saved_bytes: 0,
            weak_connections: 0,
            cleanup_operations: 0,
        }
    }

    /// 統計をリセット
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for ClosureOptimizationStats {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバルクロージャ最適化統計
static GLOBAL_CLOSURE_STATS: OnceLock<Arc<Mutex<ClosureOptimizationStats>>> = OnceLock::new();

/// グローバル統計を取得
pub fn get_closure_optimizer() -> Arc<Mutex<ClosureOptimizationStats>> {
    GLOBAL_CLOSURE_STATS
        .get_or_init(|| {
            tracing::info!("🧹 [CLOSURE] Creating global closure statistics");
            Arc::new(Mutex::new(ClosureOptimizationStats::new()))
        })
        .clone()
}

/// クロージャ作成を記録
pub fn record_closure_creation() {
    if let Ok(mut stats) = get_closure_optimizer().lock() {
        stats.total_closures_created += 1;
    }
}

/// クロージャ再利用を記録
pub fn record_closure_reuse(memory_saved: u64) {
    if let Ok(mut stats) = get_closure_optimizer().lock() {
        stats.closures_reused += 1;
        stats.memory_saved_bytes += memory_saved;
    }
}

/// WeakRef接続を記録
pub fn record_weak_connection() {
    if let Ok(mut stats) = get_closure_optimizer().lock() {
        stats.weak_connections += 1;
    }
}

/// 定期的なクリーンアップを実行 (簡略版)
pub fn perform_periodic_cleanup() {
    if let Ok(mut stats) = get_closure_optimizer().lock() {
        stats.cleanup_operations += 1;
        tracing::debug!(
            "🧹 [CLOSURE] Performed cleanup operation #{}",
            stats.cleanup_operations
        );
    }
}

/// 最適化されたSignal更新ハンドラーを取得 (簡略版)
pub fn get_optimized_signal_handler(signal_name: &str, component: &str) -> Box<dyn Fn()> {
    record_closure_creation();

    let signal_name = signal_name.to_string();
    let component = component.to_string();

    Box::new(move || {
        // 統合処理
        crate::gui::signal_optimizer::record_signal_update(&signal_name);
        crate::gui::performance_monitor::record_performance_event(
            crate::gui::performance_monitor::PerformanceEventType::SignalUpdate,
            &component,
        );
    })
}

/// WeakRef接続を作成 (簡略版 - ダミー実装)
pub fn create_weak_signal_connection<F>(_callback: F) -> Option<()>
where
    F: Fn() + 'static,
{
    record_weak_connection();
    tracing::debug!("🔗 [WEAK] Created weak signal connection (dummy)");
    Some(())
}

/// クロージャ最適化レポートを生成
pub fn generate_closure_optimization_report() -> String {
    if let Ok(stats) = get_closure_optimizer().lock() {
        let mut report = String::new();
        report.push_str("=== Closure Optimization Report (Phase 4.3 - Simplified) ===\n\n");

        report.push_str(&format!("📊 基本統計:\n"));
        report.push_str(&format!(
            "  作成されたクロージャ: {}\n",
            stats.total_closures_created
        ));
        report.push_str(&format!(
            "  再利用されたクロージャ: {}\n",
            stats.closures_reused
        ));
        report.push_str(&format!(
            "  節約されたメモリ: {:.1}KB\n",
            stats.memory_saved_bytes as f64 / 1024.0
        ));
        report.push_str(&format!("  弱い参照接続: {}\n", stats.weak_connections));
        report.push_str(&format!(
            "  クリーンアップ操作: {}\n",
            stats.cleanup_operations
        ));

        if stats.closures_reused > 0 {
            let reuse_rate =
                (stats.closures_reused as f64) / (stats.total_closures_created as f64) * 100.0;
            report.push_str(&format!("\n📈 効率性:\n"));
            report.push_str(&format!("  再利用率: {:.1}%\n", reuse_rate));
            report.push_str(&format!(
                "  メモリ節約効率: {:.1} bytes/closure\n",
                stats.memory_saved_bytes as f64 / stats.closures_reused as f64
            ));
        }

        report.push_str("\n💡 注意: この版は簡略版です。Send/Sync制約により、\n");
        report.push_str("   実際のクロージャキャッシュ機能は無効化されています。\n");

        report
    } else {
        "Error: Could not access closure optimizer statistics".to_string()
    }
}
