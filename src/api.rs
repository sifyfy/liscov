pub mod adapters; // 既存APIアダプター
pub mod auth; // YouTube認証（メンバー限定配信対応）
pub mod continuation_builder; // Continuation token生成
pub mod generic; // ジェネリックAPI統一システム
pub mod innertube;
pub mod manager; // API統合管理システム
pub mod unified_client; // 統合APIクライアント実装
pub mod websocket_server; // WebSocket API サーバー
pub mod youtube;
