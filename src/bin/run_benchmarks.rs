//! ベンチマークとテスト実行ツール (Phase 5.3-5.4)
//!
//! 長時間実行テストとパフォーマンスベンチマークを実行

use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏁 liscov ベンチマーク & テストツール");
    println!("=========================================");

    // 引数の解析
    let args: Vec<String> = std::env::args().collect();
    let test_type = if args.len() > 1 {
        args[1].as_str()
    } else {
        "all"
    };

    match test_type {
        "stability" => run_stability_tests().await?,
        "benchmark" => run_performance_benchmarks().await?,
        "all" => {
            run_stability_tests().await?;
            println!("\n{}\n", "=".repeat(50));
            run_performance_benchmarks().await?;
        }
        _ => {
            println!("使用方法: cargo run --bin run_benchmarks [stability|benchmark|all]");
            std::process::exit(1);
        }
    }

    println!("\n🎉 すべてのテストが完了しました！");
    Ok(())
}

/// 長時間実行テストを実行
async fn run_stability_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 長時間実行安定性テスト開始...");

    // モジュールパスを修正
    use liscov::stability_tests::long_running_stability_tests::{
        LongRunningTestConfig, LongRunningTestRunner,
    };

    // 長時間実行テスト（短縮版）
    let config = LongRunningTestConfig {
        duration: Duration::from_secs(15), // 15秒（実用的なテスト時間）
        sampling_interval: Duration::from_millis(100),
        max_memory_mb: 200,
        max_cpu_percent: 80.0,
        min_fps: 30.0,
    };

    let mut runner = LongRunningTestRunner::new(config);
    let results = runner.run_all_tests().await;

    // 結果の表示
    let report = runner.generate_stability_report();
    println!("{}", report);

    // 成功/失敗の判定
    let all_passed = results.iter().all(|r| r.success);
    if all_passed {
        println!("✅ 全ての安定性テストが成功しました！");
    } else {
        println!("❌ 一部のテストが失敗しました。");
        std::process::exit(1);
    }

    Ok(())
}

/// パフォーマンスベンチマークを実行
async fn run_performance_benchmarks() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 パフォーマンスベンチマーク開始...");

    // ベンチマーク実行
    if let Some(result) = liscov::gui::performance_monitor::run_performance_benchmark().await {
        // 結果の表示
        let report = liscov::gui::performance_monitor::generate_benchmark_report(&result);
        println!("{}", report);

        // パフォーマンススコアの表示
        let score = liscov::gui::performance_monitor::calculate_performance_score(&result);
        println!("🏆 総合パフォーマンススコア: {:.1}/100", score);

        // スコア評価
        if score >= 80.0 {
            println!("💫 評価: 優秀 - システムパフォーマンスは非常に良好です");
        } else if score >= 60.0 {
            println!("✅ 評価: 良好 - システムパフォーマンスは適切です");
        } else if score >= 40.0 {
            println!("⚠️ 評価: 注意 - 一部のパフォーマンス改善が必要です");
        } else {
            println!("❌ 評価: 改善必要 - パフォーマンス最適化が急務です");
        }

        // パフォーマンス警告のチェック
        let warnings = liscov::gui::performance_monitor::check_performance_warnings();
        if !warnings.is_empty() {
            println!("\n⚠️ パフォーマンス警告:");
            for warning in warnings {
                println!("  {}", warning);
            }
        }

        println!("✅ パフォーマンスベンチマークが完了しました！");
    } else {
        println!("❌ パフォーマンスベンチマークの実行に失敗しました。");
        std::process::exit(1);
    }

    Ok(())
}
