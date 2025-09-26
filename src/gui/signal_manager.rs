//! Phase 2.3: 効率的なSignal構造管理システム
//!
//! Dioxus Signal最適化による並行処理の安定化
//! - バッチ更新システム
//! - Signal依存関係の最適化
//! - デバウンス機能
//! - 競合状態の回避

use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::gui::models::GuiChatMessage;
use crate::gui::services::ServiceState;
use crate::gui::state_management::ChatStats;

/// Signal更新の種類
#[derive(Debug, Clone)]
pub enum SignalUpdateType {
    /// メッセージ追加（差分更新）
    MessageAdded(GuiChatMessage),
    /// メッセージ群追加（バッチ更新）
    MessagesAdded(Vec<GuiChatMessage>),
    /// メッセージクリア
    MessagesClear,
    /// サービス状態変更
    ServiceStateChanged(ServiceState),
    /// 接続状態変更
    ConnectionChanged(bool),
    /// 停止状態変更
    StoppingChanged(bool),
    /// 統計情報更新
    StatsUpdated(ChatStats),
}

/// Signal更新要求
#[derive(Debug, Clone)]
pub struct SignalUpdateRequest {
    pub update_type: SignalUpdateType,
    pub priority: UpdatePriority,
    pub timestamp: Instant,
    pub debounce_key: Option<String>, // デバウンス用キー
}

/// 更新優先度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UpdatePriority {
    High = 0,    // 即座に更新（UI状態変更など）
    Medium = 1,  // 通常更新（メッセージ追加など）
    Low = 2,     // 低優先度（統計情報など）
}

/// Signal依存関係定義
#[derive(Debug, Clone)]
pub struct SignalDependency {
    pub signal_id: String,
    pub depends_on: HashSet<String>,
    pub update_frequency: Duration, // 最低更新間隔
    pub last_update: Instant,
}

/// 効率的なSignal管理システム
pub struct SignalManager {
    /// 更新要求チャネル
    update_sender: mpsc::UnboundedSender<SignalUpdateRequest>,
    
    /// Signal依存関係マップ
    dependencies: Arc<Mutex<HashMap<String, SignalDependency>>>,
    
    /// デバウンス管理
    debounce_map: Arc<Mutex<HashMap<String, Instant>>>,
    
    /// 更新統計
    update_stats: Arc<Mutex<UpdateStats>>,
}

/// 更新統計情報
#[derive(Debug)]
struct UpdateStats {
    total_updates: u64,
    batched_updates: u64,
    debounced_updates: u64,
    high_priority_updates: u64,
    last_reset: Instant,
}

impl UpdateStats {
    fn new() -> Self {
        Self {
            total_updates: 0,
            batched_updates: 0,
            debounced_updates: 0,
            high_priority_updates: 0,
            last_reset: Instant::now(),
        }
    }
}

/// グローバルSignal管理システム
static GLOBAL_SIGNAL_MANAGER: OnceLock<Arc<SignalManager>> = OnceLock::new();

impl SignalManager {
    /// 新しいSignal管理システムを作成
    pub fn new() -> Self {
        let (update_sender, mut update_receiver) = mpsc::unbounded_channel();
        
        let dependencies = Arc::new(Mutex::new(HashMap::new()));
        let debounce_map = Arc::new(Mutex::new(HashMap::new()));
        let update_stats = Arc::new(Mutex::new(UpdateStats::new()));
        
        // バックグラウンドでバッチ更新処理を実行
        let deps_clone = dependencies.clone();
        let debounce_clone = debounce_map.clone();
        let stats_clone = update_stats.clone();
        
        spawn(async move {
            let mut batch_buffer: Vec<SignalUpdateRequest> = Vec::new();
            let mut batch_timer = tokio::time::interval(Duration::from_millis(16)); // 60FPS相当
            batch_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            tracing::info!("🚀 [SIGNAL_MGR] Phase 2.3 Signal batch processor started (16ms intervals)");
            
            loop {
                tokio::select! {
                    // 更新要求を受信
                    Some(update_request) = update_receiver.recv() => {
                        batch_buffer.push(update_request);
                        
                        // 高優先度の更新は即座に処理
                        if batch_buffer.last().unwrap().priority == UpdatePriority::High {
                            Self::process_batch_updates(
                                &mut batch_buffer,
                                &deps_clone,
                                &debounce_clone,
                                &stats_clone
                            ).await;
                        }
                    }
                    
                    // バッチタイマー
                    _ = batch_timer.tick() => {
                        if !batch_buffer.is_empty() {
                            Self::process_batch_updates(
                                &mut batch_buffer,
                                &deps_clone,
                                &debounce_clone,
                                &stats_clone
                            ).await;
                        }
                    }
                }
            }
        });
        
        Self {
            update_sender,
            dependencies,
            debounce_map,
            update_stats,
        }
    }
    
    /// Signal依存関係を登録
    pub fn register_signal(&self, signal_id: String, depends_on: HashSet<String>, update_frequency: Duration) {
        if let Ok(mut deps) = self.dependencies.lock() {
            deps.insert(signal_id.clone(), SignalDependency {
                signal_id,
                depends_on,
                update_frequency,
                last_update: Instant::now(),
            });
        }
    }
    
    /// Signal更新を要求（デバウンス対応）
    pub fn request_update(&self, update_type: SignalUpdateType, priority: UpdatePriority, debounce_key: Option<String>) -> Result<(), String> {
        // デバウンス処理
        if let Some(ref key) = debounce_key {
            if let Ok(mut debounce_map) = self.debounce_map.lock() {
                let now = Instant::now();
                if let Some(&last_update) = debounce_map.get(key) {
                    if now.duration_since(last_update) < Duration::from_millis(10) {
                        // デバウンス中はスキップ
                        return Ok(());
                    }
                }
                debounce_map.insert(key.clone(), now);
            }
        }
        
        let request = SignalUpdateRequest {
            update_type,
            priority,
            timestamp: Instant::now(),
            debounce_key,
        };
        
        self.update_sender.send(request)
            .map_err(|e| format!("Failed to send update request: {}", e))
    }
    
    /// バッチ更新処理（内部実装）
    async fn process_batch_updates(
        batch_buffer: &mut Vec<SignalUpdateRequest>,
        _dependencies: &Arc<Mutex<HashMap<String, SignalDependency>>>,
        _debounce_map: &Arc<Mutex<HashMap<String, Instant>>>,
        update_stats: &Arc<Mutex<UpdateStats>>,
    ) {
        if batch_buffer.is_empty() {
            return;
        }
        
        let batch_size = batch_buffer.len();
        let start_time = Instant::now();
        
        // 優先度順にソート
        batch_buffer.sort_by_key(|req| req.priority);
        
        // メッセージ更新をバッチ処理
        let mut message_batch = Vec::new();
        let mut other_updates = Vec::new();
        
        for request in batch_buffer.drain(..) {
            match request.update_type {
                SignalUpdateType::MessageAdded(msg) => {
                    message_batch.push(msg);
                }
                SignalUpdateType::MessagesAdded(mut msgs) => {
                    message_batch.append(&mut msgs);
                }
                _ => {
                    other_updates.push(request);
                }
            }
        }
        
        // メッセージバッチを処理
        if !message_batch.is_empty() {
            Self::apply_message_batch_update(message_batch).await;
        }
        
        // その他の更新を処理
        for request in other_updates {
            Self::apply_single_update(request).await;
        }
        
        let process_time = start_time.elapsed();
        
        // 統計更新
        if let Ok(mut stats) = update_stats.lock() {
            stats.total_updates += batch_size as u64;
            if batch_size > 1 {
                stats.batched_updates += 1;
            }
        }
        
        if batch_size > 5 || process_time > Duration::from_millis(5) {
            tracing::info!(
                "🚀 [SIGNAL_MGR] Processed {} updates in {:?}",
                batch_size,
                process_time
            );
        }
    }
    
    /// メッセージバッチ更新の適用
    async fn apply_message_batch_update(messages: Vec<GuiChatMessage>) {
        if messages.is_empty() {
            return;
        }
        
        let message_count = messages.len();
        
        // StateManagerに一括送信
        let state_manager = crate::gui::state_management::get_state_manager();
        for message in messages {
            let _ = state_manager.send_event(crate::gui::state_management::AppEvent::MessageAdded(message));
        }
        
        tracing::debug!(
            "🚀 [SIGNAL_MGR] Applied batch message update: {} messages",
            message_count
        );
    }
    
    /// 単一更新の適用
    async fn apply_single_update(request: SignalUpdateRequest) {
        let state_manager = crate::gui::state_management::get_state_manager();
        
        match request.update_type {
            SignalUpdateType::ServiceStateChanged(state) => {
                let _ = state_manager.send_event(crate::gui::state_management::AppEvent::ServiceStateChanged(state));
            }
            SignalUpdateType::ConnectionChanged(connected) => {
                let _ = state_manager.send_event(crate::gui::state_management::AppEvent::ConnectionChanged { is_connected: connected });
            }
            SignalUpdateType::StoppingChanged(stopping) => {
                let _ = state_manager.send_event(crate::gui::state_management::AppEvent::StoppingStateChanged { is_stopping: stopping });
            }
            SignalUpdateType::MessagesClear => {
                let _ = state_manager.send_event(crate::gui::state_management::AppEvent::MessagesCleared);
            }
            SignalUpdateType::StatsUpdated(_stats) => {
                // 統計情報の更新処理（必要に応じて実装）
            }
            _ => {
                // その他の更新は既に処理済み
            }
        }
    }
    
    /// 統計情報を取得
    pub fn get_stats(&self) -> Option<String> {
        if let Ok(stats) = self.update_stats.lock() {
            Some(format!(
                "📊 [SIGNAL_MGR] Stats: {} total, {} batched, {} debounced, {} high-priority",
                stats.total_updates,
                stats.batched_updates,
                stats.debounced_updates,
                stats.high_priority_updates
            ))
        } else {
            None
        }
    }
}

/// グローバルSignal管理システムを取得
pub fn get_signal_manager() -> &'static Arc<SignalManager> {
    GLOBAL_SIGNAL_MANAGER.get_or_init(|| {
        tracing::info!("🚀 [SIGNAL_MGR] Phase 2.3 Global Signal Manager initialized");
        Arc::new(SignalManager::new())
    })
}

/// 効率的なSignal更新フック
pub fn use_optimized_signals() -> OptimizedSignalHandle {
    let signal_manager = get_signal_manager();
    
    // Signal依存関係を登録
    let mut message_deps = HashSet::new();
    message_deps.insert("state_manager".to_string());
    
    signal_manager.register_signal(
        "messages".to_string(),
        message_deps,
        Duration::from_millis(16), // 60FPS相当
    );
    
    OptimizedSignalHandle {
        manager: signal_manager.clone(),
    }
}

/// 最適化されたSignalハンドル
pub struct OptimizedSignalHandle {
    manager: Arc<SignalManager>,
}

impl OptimizedSignalHandle {
    /// 高優先度でメッセージ追加
    pub fn add_message_high_priority(&self, message: GuiChatMessage) {
        let _ = self.manager.request_update(
            SignalUpdateType::MessageAdded(message),
            UpdatePriority::High,
            None,
        );
    }
    
    /// デバウンス付きでメッセージ追加
    pub fn add_message_debounced(&self, message: GuiChatMessage, debounce_key: String) {
        let _ = self.manager.request_update(
            SignalUpdateType::MessageAdded(message),
            UpdatePriority::Medium,
            Some(debounce_key),
        );
    }
    
    /// バッチでメッセージ追加
    pub fn add_messages_batch(&self, messages: Vec<GuiChatMessage>) {
        let _ = self.manager.request_update(
            SignalUpdateType::MessagesAdded(messages),
            UpdatePriority::Medium,
            None,
        );
    }
    
    /// サービス状態変更（高優先度）
    pub fn update_service_state(&self, state: ServiceState) {
        let _ = self.manager.request_update(
            SignalUpdateType::ServiceStateChanged(state),
            UpdatePriority::High,
            Some("service_state".to_string()),
        );
    }
    
    /// 接続状態変更（高優先度）
    pub fn update_connection_state(&self, connected: bool) {
        let _ = self.manager.request_update(
            SignalUpdateType::ConnectionChanged(connected),
            UpdatePriority::High,
            Some("connection_state".to_string()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signal_manager_creation() {
        let manager = SignalManager::new();
        assert!(manager.update_sender.send(SignalUpdateRequest {
            update_type: SignalUpdateType::MessagesClear,
            priority: UpdatePriority::High,
            timestamp: Instant::now(),
            debounce_key: None,
        }).is_ok());
    }

    #[test]
    fn test_signal_dependency_registration() {
        let manager = SignalManager::new();
        let mut deps = HashSet::new();
        deps.insert("test_dep".to_string());
        
        manager.register_signal(
            "test_signal".to_string(),
            deps,
            Duration::from_millis(100),
        );
        
        // 依存関係が正しく登録されたかテスト
        assert!(manager.dependencies.lock().unwrap().contains_key("test_signal"));
    }
}