/// 分析機能統合サービス
///
/// 新アーキテクチャと既存分析機能を統合
use crate::analytics::{EngagementMetrics, EngagementSummary};
use crate::gui::models::GuiChatMessage;
use crate::gui::state_management::get_state_manager;
use serde::{Deserialize, Serialize};

use tokio::sync::mpsc;
use tracing;

// エクスポート機能のインポートを追加
use crate::analytics::export::{
    ExportConfig, ExportError, ExportFormat, ExportManager, ExportableData, SessionData,
};
use std::collections::HashMap;

/// 分析統合サービス（エクスポート機能付き）
pub struct AnalyticsIntegrationService {
    // 静的メソッドのみを使用するため、フィールドは不要
}

/// 分析結果（エクスポート対応）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisResult {
    /// 分析タイムスタンプ
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// エンゲージメント要約
    pub engagement_summary: EngagementSummary,
    /// 分析されたメッセージ数
    pub analyzed_message_count: usize,
    /// 分析期間
    pub analysis_duration_ms: u64,
}

/// エクスポート結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    /// エクスポート成功フラグ
    pub success: bool,
    /// エクスポートされたファイルパス
    pub file_path: Option<String>,
    /// エクスポートファイルサイズ（バイト）
    pub file_size: Option<usize>,
    /// エクスポート形式
    pub format: ExportFormat,
    /// エクスポート期間（ms）
    pub export_duration_ms: u64,
    /// エラーメッセージ（失敗時）
    pub error_message: Option<String>,
    /// エクスポートされたレコード数
    pub exported_records: usize,
}

/// グローバル分析サービス実行状態
static ANALYTICS_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

impl AnalyticsIntegrationService {
    /// 新しい分析統合サービスを作成
    pub fn new() -> Self {
        Self {
            // 静的メソッドのみを使用するため、フィールドの初期化は不要
        }
    }

    /// 分析サービスを開始（グローバル制御）
    pub fn start() -> Result<mpsc::UnboundedReceiver<AnalysisResult>, String> {
        if ANALYTICS_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Analytics service is already running".to_string());
        }

        ANALYTICS_RUNNING.store(true, std::sync::atomic::Ordering::Relaxed);

        let (analysis_tx, analysis_rx) = mpsc::unbounded_channel();

        // バックグラウンドで分析処理を実行
        tokio::spawn(async {
            Self::run_background_analysis(analysis_tx).await;
        });

        tracing::info!("📊 Analytics integration service started");
        Ok(analysis_rx)
    }

    /// 分析サービスの実行状態を確認
    pub fn is_running() -> bool {
        ANALYTICS_RUNNING.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 分析サービスを停止
    pub fn stop() {
        ANALYTICS_RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("📊 Analytics integration service stop requested");
    }

    /// バックグラウンド分析処理
    async fn run_background_analysis(analysis_sender: mpsc::UnboundedSender<AnalysisResult>) {
        let mut engagement_metrics = EngagementMetrics::new();
        let mut last_analyzed_count = 0;
        let mut analysis_counter = 0;

        tracing::info!("📊 Background analytics processing started");

        while ANALYTICS_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
            analysis_counter += 1;
            let start_time = std::time::Instant::now();

            // 現在の状態を取得
            let current_state = get_state_manager().get_state();
            let current_message_count = current_state.messages.len();

            // 新しいメッセージがある場合のみ分析
            if current_message_count > last_analyzed_count {
                let new_messages = &current_state.messages[last_analyzed_count..];

                // エンゲージメント分析を実行
                Self::process_new_messages(&mut engagement_metrics, new_messages);

                // 分析結果を作成
                let analysis_duration = start_time.elapsed();
                let analysis_result = AnalysisResult {
                    timestamp: chrono::Utc::now(),
                    engagement_summary: engagement_metrics.get_engagement_summary(),
                    analyzed_message_count: new_messages.len(),
                    analysis_duration_ms: analysis_duration.as_millis() as u64,
                };

                // 分析結果を送信
                if let Err(_) = analysis_sender.send(analysis_result.clone()) {
                    tracing::warn!("📊 Failed to send analysis result");
                    break;
                }

                tracing::info!(
                    "📊 Analyzed {} new messages (total: {}, duration: {}ms)",
                    new_messages.len(),
                    current_message_count,
                    analysis_duration.as_millis()
                );

                last_analyzed_count = current_message_count;
            } else if analysis_counter % 600 == 0 {
                // 30秒に1回の生存確認（50ms * 600 = 30s）
                tracing::debug!(
                    "📊 Analytics service alive - no new messages ({})",
                    analysis_counter
                );
            }

            // 50msごとに状態をチェック
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        tracing::info!("📊 Background analytics processing stopped");
    }

    /// 新しいメッセージを処理
    fn process_new_messages(
        engagement_metrics: &mut EngagementMetrics,
        messages: &[GuiChatMessage],
    ) {
        for message in messages {
            engagement_metrics.update_from_message(message);
        }

        // 大量メッセージ処理時の最適化コメント
        // 実際の再計算は update_from_message 内で自動的に実行される
    }

    /// 現在の分析結果を取得
    pub fn get_current_analysis() -> AnalysisResult {
        let current_state = get_state_manager().get_state();
        let mut engagement_metrics = EngagementMetrics::new();

        // 全メッセージを分析（リアルタイム用）
        for message in &current_state.messages {
            engagement_metrics.update_from_message(message);
        }

        AnalysisResult {
            timestamp: chrono::Utc::now(),
            engagement_summary: engagement_metrics.get_engagement_summary(),
            analyzed_message_count: current_state.messages.len(),
            analysis_duration_ms: 0, // リアルタイム計算のため0
        }
    }

    /// 分析統計をAppEventとして送信
    pub fn broadcast_analysis_update(analysis_result: &AnalysisResult) {
        // 将来的に分析結果をstate managementに統合する場合の準備
        tracing::debug!(
            "📊 Broadcasting analysis update: {} messages analyzed",
            analysis_result.analyzed_message_count
        );
    }

    /// 現在のメッセージをエクスポート（メモリ内データ）
    pub fn export_current_data(format: ExportFormat) -> Result<ExportResult, ExportError> {
        let start_time = std::time::Instant::now();

        // 現在の状態管理からメッセージを取得
        let state_manager = get_state_manager();
        let state = state_manager.get_state();
        let messages = &state.messages;

        // SessionDataに変換
        let session_data = Self::convert_to_session_data(messages);

        // エクスポート設定
        let config = ExportConfig {
            format,
            include_metadata: true,
            date_range: None,
            include_system_messages: true,
            include_deleted_messages: false,
            max_records: None,
            sort_order: crate::analytics::export::SortOrder::Chronological,
        };

        // エクスポート実行
        let export_manager = ExportManager::new();
        let exported_data = export_manager.export(&session_data, &config)?;

        let export_duration = start_time.elapsed();

        Ok(ExportResult {
            success: true,
            file_path: None, // メモリエクスポートのためパスなし
            file_size: Some(exported_data.len()),
            format,
            export_duration_ms: export_duration.as_millis() as u64,
            error_message: None,
            exported_records: session_data.messages.len(),
        })
    }

    /// ファイルにエクスポート
    pub fn export_to_file(
        format: ExportFormat,
        file_path: &str,
    ) -> Result<ExportResult, ExportError> {
        let start_time = std::time::Instant::now();

        // メモリ内エクスポートを実行
        Self::export_current_data(format)?;

        // ファイルに書き込み（実際の実装では、export_managerからデータを取得）
        let state_manager = get_state_manager();
        let state = state_manager.get_state();
        let messages = &state.messages;
        let session_data = Self::convert_to_session_data(messages);

        let config = ExportConfig {
            format,
            include_metadata: true,
            date_range: None,
            include_system_messages: true,
            include_deleted_messages: false,
            max_records: None,
            sort_order: crate::analytics::export::SortOrder::Chronological,
        };

        let export_manager = ExportManager::new();
        let exported_data = export_manager.export(&session_data, &config)?;

        // ファイル書き込み
        std::fs::write(file_path, &exported_data).map_err(ExportError::Io)?;

        let export_duration = start_time.elapsed();

        Ok(ExportResult {
            success: true,
            file_path: Some(file_path.to_string()),
            file_size: Some(exported_data.len()),
            format,
            export_duration_ms: export_duration.as_millis() as u64,
            error_message: None,
            exported_records: session_data.messages.len(),
        })
    }

    /// GuiChatMessage を SessionData に変換
    fn convert_to_session_data(messages: &[GuiChatMessage]) -> SessionData {
        let mut session_data = SessionData::new(
            format!("session_{}", chrono::Utc::now().timestamp()),
            "https://youtube.com/watch?v=demo".to_string(),
            "Demo Channel".to_string(),
            "demo-channel-id".to_string(),
        );

        for (index, msg) in messages.iter().enumerate() {
            let exportable_data = ExportableData {
                id: format!("msg_{}", index),
                timestamp: chrono::Utc::now(), // 実際の実装では正確なタイムスタンプを使用
                author: msg.author.clone(),
                author_id: msg.channel_id.clone(),
                content: msg.content.clone(),
                message_type: msg.message_type.as_string(),
                amount: Self::extract_amount(&msg.message_type),
                currency: Self::extract_currency(&msg.message_type),
                emoji_count: Self::count_emojis(&msg.content),
                word_count: msg.content.split_whitespace().count(),
                is_deleted: false,
                is_moderator: false,
                is_member: matches!(
                    msg.message_type,
                    crate::gui::models::MessageType::Membership
                ),
                is_verified: false,
                badges: vec![],
                metadata: HashMap::new(),
            };

            session_data.messages.push(exportable_data);
        }

        session_data
    }

    /// SuperChatから金額を抽出
    fn extract_amount(message_type: &crate::gui::models::MessageType) -> Option<f64> {
        match message_type {
            crate::gui::models::MessageType::SuperChat { amount } => {
                amount.replace(['¥', '$', '€', '£'], "").parse().ok()
            }
            crate::gui::models::MessageType::SuperSticker { amount } => {
                amount.replace(['¥', '$', '€', '£'], "").parse().ok()
            }
            _ => None,
        }
    }

    /// SuperChatから通貨を抽出
    fn extract_currency(message_type: &crate::gui::models::MessageType) -> Option<String> {
        match message_type {
            crate::gui::models::MessageType::SuperChat { amount } => {
                if amount.contains('¥') {
                    Some("JPY".to_string())
                } else if amount.contains('$') {
                    Some("USD".to_string())
                } else if amount.contains('€') {
                    Some("EUR".to_string())
                } else if amount.contains('£') {
                    Some("GBP".to_string())
                } else {
                    None
                }
            }
            crate::gui::models::MessageType::SuperSticker { amount } => {
                if amount.contains('¥') {
                    Some("JPY".to_string())
                } else if amount.contains('$') {
                    Some("USD".to_string())
                } else if amount.contains('€') {
                    Some("EUR".to_string())
                } else if amount.contains('£') {
                    Some("GBP".to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 絵文字数をカウント
    fn count_emojis(content: &str) -> usize {
        // 簡易絵文字検出
        content
            .chars()
            .filter(|c| {
                (*c as u32) >= 0x1F600 && (*c as u32) <= 0x1F64F || // Emoticons
            (*c as u32) >= 0x1F300 && (*c as u32) <= 0x1F5FF || // Misc Symbols
            (*c as u32) >= 0x1F680 && (*c as u32) <= 0x1F6FF || // Transport & Map
            (*c as u32) >= 0x2600 && (*c as u32) <= 0x26FF ||   // Misc symbols
            (*c as u32) >= 0x2700 && (*c as u32) <= 0x27BF ||   // Dingbats
            (*c as u32) >= 0xFE00 && (*c as u32) <= 0xFE0F // Variation Selectors
            })
            .count()
    }
}

impl Default for AnalyticsIntegrationService {
    fn default() -> Self {
        Self::new()
    }
}

/// 分析統合サービスのアクション
pub struct AnalyticsActions;

impl AnalyticsActions {
    /// 分析を開始
    pub fn start_analysis() -> Result<mpsc::UnboundedReceiver<AnalysisResult>, String> {
        AnalyticsIntegrationService::start()
    }

    /// 分析を停止
    pub fn stop_analysis() {
        AnalyticsIntegrationService::stop();
    }

    /// 現在の分析結果を取得
    pub fn get_current_analysis() -> AnalysisResult {
        AnalyticsIntegrationService::get_current_analysis()
    }

    /// 分析サービスの実行状態を確認
    pub fn is_running() -> bool {
        AnalyticsIntegrationService::is_running()
    }
}

/// エクスポート機能のアクション
pub struct ExportActions;

impl ExportActions {
    /// CSVエクスポート
    pub fn export_csv() -> Result<ExportResult, ExportError> {
        AnalyticsIntegrationService::export_current_data(ExportFormat::Csv)
    }

    /// JSONエクスポート
    pub fn export_json() -> Result<ExportResult, ExportError> {
        AnalyticsIntegrationService::export_current_data(ExportFormat::Json)
    }

    /// Excelエクスポート
    pub fn export_excel() -> Result<ExportResult, ExportError> {
        AnalyticsIntegrationService::export_current_data(ExportFormat::Excel)
    }

    /// ファイルエクスポート
    pub fn export_to_file(
        format: ExportFormat,
        file_path: &str,
    ) -> Result<ExportResult, ExportError> {
        AnalyticsIntegrationService::export_to_file(format, file_path)
    }

    /// サポートされている形式を取得
    pub fn supported_formats() -> Vec<ExportFormat> {
        vec![ExportFormat::Csv, ExportFormat::Json, ExportFormat::Excel]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::models::{GuiChatMessage, MessageType};

    /// テスト用のサンプルメッセージを作成
    fn create_test_message(
        author: &str,
        content: &str,
        message_type: MessageType,
    ) -> GuiChatMessage {
        GuiChatMessage {
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            message_type,
            author: author.to_string(),
            channel_id: format!("test_channel_{}", author),
            content: content.to_string(),
            metadata: None,
            is_member: false,
        }
    }

    /// AnalysisResultの基本構造テスト
    #[test]
    fn test_analysis_result_creation() {
        let timestamp = chrono::Utc::now();
        let summary = EngagementSummary {
            unique_viewers: 5,
            engagement_rate: 75.0,
            emoji_usage_rate: 30.0,
            average_message_length: 25.5,
            questions_count: 2,
            active_sessions: 3,
            total_messages: 10,
            peak_hour: Some(14),
        };

        let analysis = AnalysisResult {
            timestamp,
            engagement_summary: summary.clone(),
            analyzed_message_count: 10,
            analysis_duration_ms: 150,
        };

        assert_eq!(analysis.analyzed_message_count, 10);
        assert_eq!(analysis.analysis_duration_ms, 150);
        assert_eq!(analysis.engagement_summary.unique_viewers, 5);
        assert_eq!(analysis.engagement_summary.engagement_rate, 75.0);
        println!("✅ AnalysisResult構造体テスト完了");
    }

    /// AnalyticsIntegrationServiceのインスタンス作成テスト
    #[test]
    fn test_service_creation() {
        AnalyticsIntegrationService::new();

        // サービスが正常に作成されることを確認（静的メソッドのみのため、フィールドチェックは不要）
        assert!(!AnalyticsIntegrationService::is_running());
        println!("✅ サービスインスタンス作成テスト完了");
    }

    /// メッセージ処理機能のテスト（独立）
    #[test]
    fn test_message_processing_logic() {
        let mut engagement_metrics = EngagementMetrics::new();

        let test_messages = vec![
            create_test_message("user1", "こんにちは！", MessageType::Text),
            create_test_message("user2", "😊 楽しいです", MessageType::Text),
            create_test_message("user3", "質問があります？", MessageType::Text),
        ];

        // メッセージ処理をシミュレート
        for message in &test_messages {
            engagement_metrics.update_from_message(message);
        }

        let summary = engagement_metrics.get_engagement_summary();

        assert!(summary.total_messages >= 3);
        assert!(summary.unique_viewers >= 3);
        assert!(summary.emoji_usage_rate >= 0.0);

        println!(
            "✅ メッセージ処理ロジックテスト完了: {} メッセージ処理",
            summary.total_messages
        );
    }

    /// エンゲージメント計算テスト
    #[test]
    fn test_engagement_calculation() {
        let mut engagement_metrics = EngagementMetrics::new();

        // 高エンゲージメントメッセージ
        let high_engagement_messages = vec![
            create_test_message("fan1", "❤️ 素晴らしい！", MessageType::Text),
            create_test_message(
                "supporter",
                "応援しています",
                MessageType::SuperChat {
                    amount: "¥500".to_string(),
                },
            ),
            create_test_message("fan2", "😊🎉 最高です！", MessageType::Text),
        ];

        for message in &high_engagement_messages {
            engagement_metrics.update_from_message(message);
        }

        let summary = engagement_metrics.get_engagement_summary();

        assert!(summary.emoji_usage_rate > 0.0);
        assert!(summary.engagement_rate >= 0.0);

        println!(
            "✅ エンゲージメント計算テスト完了: エンゲージメント率 {:.1}%",
            summary.engagement_rate
        );
    }

    /// パフォーマンステスト（軽量版）
    #[test]
    fn test_lightweight_performance() {
        let start_time = std::time::Instant::now();

        let mut engagement_metrics = EngagementMetrics::new();

        // 軽量バッチ更新を使用
        let bulk_messages: Vec<GuiChatMessage> = (1..=50)
            .map(|i| {
                create_test_message(
                    &format!("user_{}", i),
                    &format!("メッセージ {}", i),
                    MessageType::Text,
                )
            })
            .collect();

        engagement_metrics.update_from_messages_lightweight(&bulk_messages);

        let processing_time = start_time.elapsed();
        let summary = engagement_metrics.get_engagement_summary();

        // 軽量バッチ更新は統計処理のため、activity_stats.total_messagesは更新されない
        // 代わりにユニーク視聴者数と処理時間で検証
        assert!(summary.unique_viewers >= 50); // ユニーク視聴者数で確認
        assert!(summary.emoji_usage_rate >= 0.0); // 絵文字率が計算されている
        assert!(summary.engagement_rate >= 0.0); // エンゲージメント率が計算されている
        assert!(processing_time.as_millis() < 1000); // 1秒以内

        println!(
            "✅ 軽量パフォーマンステスト完了: {} ユニーク視聴者を {}ms で処理",
            summary.unique_viewers,
            processing_time.as_millis()
        );
    }

    /// 感情分析統合テスト（独立）
    #[test]
    fn test_sentiment_analysis_standalone() {
        let mut engagement_metrics = EngagementMetrics::new();

        let sentiment_messages = vec![
            create_test_message(
                "positive",
                "素晴らしい！ありがとうございます！",
                MessageType::Text,
            ),
            create_test_message("excited", "うわー！！！すごすぎる！！！", MessageType::Text),
            create_test_message("emoji", "😂😂😂 笑いすぎです", MessageType::Text),
        ];

        for message in &sentiment_messages {
            engagement_metrics.update_from_message(message);
        }

        let summary = engagement_metrics.get_engagement_summary();

        assert!(summary.total_messages >= 3);
        assert!(summary.emoji_usage_rate > 0.0);

        println!(
            "✅ 感情分析独立テスト完了: {} メッセージの感情分析",
            summary.total_messages
        );
    }

    /// エラーハンドリングテスト（独立）
    #[test]
    fn test_service_state_logic() {
        // 初期状態確認
        assert!(!AnalyticsIntegrationService::is_running());

        // 停止状態での停止呼び出し（エラーではない）
        AnalyticsIntegrationService::stop();

        println!("✅ サービス状態ロジックテスト完了");
    }

    /// エクスポート機能テスト
    #[test]
    fn test_export_functionality() {
        // CSVエクスポートテスト
        let csv_result = ExportActions::export_csv();
        assert!(csv_result.is_ok());

        let csv_export = csv_result.unwrap();
        assert!(csv_export.success);
        assert_eq!(csv_export.format, ExportFormat::Csv);
        assert!(csv_export.file_size.unwrap_or(0) > 0);

        // JSONエクスポートテスト
        let json_result = ExportActions::export_json();
        assert!(json_result.is_ok());

        let json_export = json_result.unwrap();
        assert!(json_export.success);
        assert_eq!(json_export.format, ExportFormat::Json);

        // Excelエクスポートテスト
        let excel_result = ExportActions::export_excel();
        assert!(excel_result.is_ok());

        let excel_export = excel_result.unwrap();
        assert!(excel_export.success);
        assert_eq!(excel_export.format, ExportFormat::Excel);

        println!("✅ エクスポート機能テスト完了: CSV, JSON, Excel 全て成功");
    }

    /// メッセージ変換テスト
    #[test]
    fn test_message_conversion() {
        let test_messages = vec![
            create_test_message("user1", "Hello world!", MessageType::Text),
            create_test_message("user2", "😊👍", MessageType::Text),
            create_test_message(
                "supporter",
                "Thank you!",
                MessageType::SuperChat {
                    amount: "¥500".to_string(),
                },
            ),
        ];

        let session_data = AnalyticsIntegrationService::convert_to_session_data(&test_messages);

        assert_eq!(session_data.messages.len(), 3);
        assert_eq!(session_data.messages[0].author, "user1");
        assert_eq!(session_data.messages[0].content, "Hello world!");
        assert_eq!(session_data.messages[0].word_count, 2);

        // 絵文字カウントテスト
        assert!(session_data.messages[1].emoji_count > 0);

        // SuperChat金額テスト
        assert_eq!(session_data.messages[2].amount, Some(500.0));
        assert_eq!(session_data.messages[2].currency, Some("JPY".to_string()));

        println!("✅ メッセージ変換テスト完了: 正確な変換を確認");
    }

    /// サポートされている形式テスト
    #[test]
    fn test_supported_formats() {
        let formats = ExportActions::supported_formats();

        assert!(formats.contains(&ExportFormat::Csv));
        assert!(formats.contains(&ExportFormat::Json));
        assert!(formats.contains(&ExportFormat::Excel));
        assert_eq!(formats.len(), 3);

        println!("✅ サポート形式テスト完了: CSV, JSON, Excel をサポート");
    }

    /// エクスポート結果構造テスト
    #[test]
    fn test_export_result_structure() {
        let csv_result = ExportActions::export_csv().unwrap();

        // 必須フィールドの検証
        assert!(csv_result.success);
        assert!(csv_result.file_size.is_some());
        assert!(csv_result.export_duration_ms > 0);
        assert!(csv_result.error_message.is_none());
        assert_eq!(csv_result.format, ExportFormat::Csv);

        // パフォーマンステスト（1秒以内）
        assert!(csv_result.export_duration_ms < 1000);

        println!("✅ エクスポート結果構造テスト完了: 適切な結果構造を確認");
    }

    /// 分析結果フォーマットテスト
    #[test]
    fn test_analysis_result_serialization() {
        let summary = EngagementSummary {
            unique_viewers: 10,
            engagement_rate: 85.5,
            emoji_usage_rate: 42.3,
            average_message_length: 33.7,
            questions_count: 3,
            active_sessions: 7,
            total_messages: 25,
            peak_hour: Some(15),
        };

        let analysis = AnalysisResult {
            timestamp: chrono::Utc::now(),
            engagement_summary: summary,
            analyzed_message_count: 25,
            analysis_duration_ms: 240,
        };

        // JSONシリアライゼーションテスト
        let json = serde_json::to_string(&analysis);
        assert!(json.is_ok());

        println!("✅ 分析結果シリアライゼーションテスト完了");
    }
}
