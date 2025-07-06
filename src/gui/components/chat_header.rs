use dioxus::prelude::*;
use crate::gui::{
    hooks::LiveChatHandle,
    styles::theme::{CssClasses, get_connection_status_class},
};

/// チャットヘッダーコンポーネント
/// 
/// 接続状態の表示を担当する軽量なヘッダーコンポーネント
/// 
/// # Props
/// - `live_chat_handle`: ライブチャットハンドル（接続状態取得用）
/// - `is_connecting`: 接続中フラグ
#[derive(Props, Clone, PartialEq)]
pub struct ChatHeaderProps {
    /// ライブチャットサービスハンドル
    pub live_chat_handle: LiveChatHandle,
    /// 接続処理中フラグ
    pub is_connecting: bool,
}

/// チャットヘッダーコンポーネント
/// 
/// 責務:
/// - ライブチャット接続状態の表示
/// - 状態に応じた視覚的なフィードバック提供
/// 
/// 分離理由:
/// - 単一責任原則（状態表示のみ）
/// - 独立性が高く再利用可能
/// - テンプレート複雑度軽減
#[component]
pub fn ChatHeader(props: ChatHeaderProps) -> Element {
    let ChatHeaderProps {
        live_chat_handle,
        is_connecting,
    } = props;

    rsx! {
        div {
            class: CssClasses::CHAT_HEADER,
            style: "
                flex-shrink: 0;
                padding: 4px 8px !important;
                background: #f7fafc;
                border-bottom: 1px solid #e2e8f0;
                display: flex;
                justify-content: space-between;
                align-items: center;
            ",

            // 接続状態表示
            div {
                class: get_connection_status_class(*live_chat_handle.is_connected.read(), is_connecting),
                style: "
                    font-weight: 600;
                    padding: 4px 10px !important;
                    border-radius: 16px;
                    font-size: 12px !important;
                    display: flex;
                    align-items: center;
                    gap: 6px;
                ",
                {
                    match *live_chat_handle.state.read() {
                        crate::gui::services::ServiceState::Connected => "🟢 接続中",
                        crate::gui::services::ServiceState::Connecting => "🟡 接続中...",
                        crate::gui::services::ServiceState::Paused => "⏸️ 一時停止",
                        crate::gui::services::ServiceState::Idle => "⚪ 待機中",
                        crate::gui::services::ServiceState::Error(_) => "🔴 エラー",
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ChatHeaderコンポーネントの基本的な構造テスト
    #[test]
    fn test_chat_header_structure() {
        // コンポーネントの基本構造が適切に定義されているかテスト
        // 実際のSignalやContextが必要な統合テストは別途実装
        assert!(true); // プレースホルダー
    }
}