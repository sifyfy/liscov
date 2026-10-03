use super::*;
use crate::core::GiftDetails;
use chrono::{DateTime, Utc};

use crate::core::{ChatMessage, MessageMetadata, MessageType, SuperChatColors};

// ========================================================================
// tier_from_header_color (07_revenue.md「Tier別集計」の表)
// ========================================================================

#[test]
fn tier_from_header_color_matches_spec_table() {
    let cases = [
        ("#1565C0", SuperChatTier::Blue),
        ("#00B8D4", SuperChatTier::Cyan),
        ("#00BFA5", SuperChatTier::Green),
        ("#FFB300", SuperChatTier::Yellow),
        ("#E65100", SuperChatTier::Orange),
        ("#C2185B", SuperChatTier::Magenta),
        ("#D00000", SuperChatTier::Red),
    ];
    for (color, tier) in cases {
        assert_eq!(tier_from_header_color(Some(color)), Some(tier), "{color}");
    }
}

#[test]
fn tier_from_header_color_ignores_case() {
    assert_eq!(
        tier_from_header_color(Some("#00bfa5")),
        Some(SuperChatTier::Green)
    );
}

#[test]
fn tier_from_header_color_not_in_table_is_unknown() {
    // 本文の背景色（緑 #1DE9B6・赤 #E62117）はヘッダー色ではない
    assert_eq!(tier_from_header_color(Some("#1DE9B6")), None);
    assert_eq!(tier_from_header_color(Some("#E62117")), None);
    assert_eq!(tier_from_header_color(Some("")), None);
}

#[test]
fn tier_from_header_color_without_color_is_unknown() {
    assert_eq!(tier_from_header_color(None), None);
}

#[test]
fn tier_stats_record_unknown() {
    let mut stats = SuperChatTierStats::default();
    stats.record(None);
    stats.record(Some(SuperChatTier::Green));
    assert_eq!(stats.tier_unknown, 1);
    assert_eq!(stats.tier_green, 1);
    assert_eq!(stats.total(), 2);
}

// ========================================================================
// SuperChatTierStats (07_revenue.md: Tier統計)
// ========================================================================

#[test]
fn tier_stats_increment_and_total() {
    let mut stats = SuperChatTierStats::default();
    assert_eq!(stats.total(), 0);

    stats.increment(SuperChatTier::Red);
    stats.increment(SuperChatTier::Red);
    stats.increment(SuperChatTier::Blue);
    stats.increment(SuperChatTier::Yellow);

    assert_eq!(stats.tier_red, 2);
    assert_eq!(stats.tier_blue, 1);
    assert_eq!(stats.tier_yellow, 1);
    assert_eq!(stats.total(), 4);
}

#[test]
fn tier_stats_default_all_zero() {
    let stats = SuperChatTierStats::default();
    assert_eq!(stats.tier_red, 0);
    assert_eq!(stats.tier_magenta, 0);
    assert_eq!(stats.tier_orange, 0);
    assert_eq!(stats.tier_yellow, 0);
    assert_eq!(stats.tier_green, 0);
    assert_eq!(stats.tier_cyan, 0);
    assert_eq!(stats.tier_blue, 0);
    assert_eq!(stats.total(), 0);
}

// ========================================================================
// export_to_csv (07_revenue.md: CSVエクスポート)
// ========================================================================

fn make_test_export_data() -> SessionExportData {
    SessionExportData {
        metadata: SessionMetadata {
            session_id: "test-session-1".to_string(),
            stream_title: Some("Test Stream".to_string()),
            stream_url: Some("https://youtube.com/watch?v=test".to_string()),
            broadcaster_name: Some("TestChannel".to_string()),
            broadcaster_channel_id: Some("UC_test".to_string()),
            start_time: "2025-01-14T14:00:00Z".to_string(),
            end_time: Some("2025-01-14T16:00:00Z".to_string()),
            export_time: "2025-01-14T17:00:00Z".to_string(),
        },
        messages: vec![
            ExportMessage {
                id: "msg1".to_string(),
                timestamp: "14:00:01".to_string(),
                author: "User1".to_string(),
                author_id: "UC_user1".to_string(),
                content: "Hello".to_string(),
                message_type: "text".to_string(),
                amount_display: None,
                tier: None,
                is_moderator: Some(false),
                is_member: false,
                is_verified: Some(false),
                badges: Some(vec![]),
            },
            ExportMessage {
                id: "msg2".to_string(),
                timestamp: "14:00:05".to_string(),
                author: "User2".to_string(),
                author_id: "UC_user2".to_string(),
                content: "Super Chat!".to_string(),
                message_type: "superchat".to_string(),
                amount_display: Some("$10.00".to_string()),
                tier: Some(SuperChatTier::Yellow),
                is_moderator: Some(false),
                is_member: true,
                is_verified: Some(false),
                badges: Some(vec!["member".to_string()]),
            },
        ],
        statistics: SessionStatistics {
            total_messages: 2,
            unique_viewers: 2,
            super_chat_count: 1,
            super_chat_by_tier: SuperChatTierStats::default(),
            membership_count: 0,
            gifts: GiftStats::default(),
        },
    }
}

#[test]
fn csv_export_with_metadata() {
    let data = make_test_export_data();
    let config = ExportConfig {
        format: "csv".to_string(),
        include_metadata: true,
        include_system_messages: false,
        max_records: None,
        sort_order: None,
    };

    let csv = export_to_csv(&data, &config).unwrap();

    assert!(csv.starts_with("# Metadata\n"));
    assert!(csv.contains("# Session ID,test-session-1"));
    assert!(csv.contains("# Stream Title,Test Stream"));
    assert!(csv.contains("# Channel,TestChannel"));
    assert!(csv.contains("# Total Messages,2"));
    assert!(csv.contains("# Unique Viewers,2"));
    assert!(csv.contains("# SuperChat Count,1"));
    assert!(csv.contains("id,timestamp,author,author_id,content,message_type,amount_display,tier,is_moderator,is_member,is_verified,badges\n"));
    assert!(csv.contains("\"msg1\""));
    assert!(csv.contains("\"msg2\""));
}

#[test]
fn csv_export_without_metadata() {
    let data = make_test_export_data();
    let config = ExportConfig {
        format: "csv".to_string(),
        include_metadata: false,
        include_system_messages: false,
        max_records: None,
        sort_order: None,
    };

    let csv = export_to_csv(&data, &config).unwrap();

    assert!(!csv.contains("# Metadata"));
    assert!(csv.starts_with("id,timestamp,"));
}

#[test]
fn csv_export_header_matches_spec() {
    let data = make_test_export_data();
    let config = ExportConfig {
        format: "csv".to_string(),
        include_metadata: false,
        include_system_messages: false,
        max_records: None,
        sort_order: None,
    };

    let csv = export_to_csv(&data, &config).unwrap();
    let header_line = csv.lines().next().unwrap();
    assert_eq!(
        header_line,
        "id,timestamp,author,author_id,content,message_type,amount_display,tier,is_moderator,is_member,is_verified,badges"
    );
}

#[test]
fn csv_export_superchat_row_has_tier() {
    let data = make_test_export_data();
    let config = ExportConfig {
        format: "csv".to_string(),
        include_metadata: false,
        include_system_messages: false,
        max_records: None,
        sort_order: None,
    };

    let csv = export_to_csv(&data, &config).unwrap();
    let superchat_line = csv.lines().find(|l| l.contains("msg2")).unwrap();
    assert!(superchat_line.contains("yellow"));
    assert!(superchat_line.contains("$10.00"));
}

// ========================================================================
// export_to_json (07_revenue.md: JSONエクスポート)
// ========================================================================

#[test]
fn json_export_with_metadata() {
    let data = make_test_export_data();
    let config = ExportConfig {
        format: "json".to_string(),
        include_metadata: true,
        include_system_messages: false,
        max_records: None,
        sort_order: None,
    };

    let json = export_to_json(&data, &config).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed.get("metadata").is_some());
    assert!(parsed.get("messages").is_some());
    assert!(parsed.get("statistics").is_some());
    assert_eq!(parsed["metadata"]["session_id"], "test-session-1");
}

#[test]
fn json_export_without_metadata_returns_messages_only() {
    let data = make_test_export_data();
    let config = ExportConfig {
        format: "json".to_string(),
        include_metadata: false,
        include_system_messages: false,
        max_records: None,
        sort_order: None,
    };

    let json = export_to_json(&data, &config).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

// ========================================================================
// RevenueAnalytics default (07_revenue.md)
// ========================================================================

#[test]
fn revenue_analytics_default() {
    let analytics = RevenueAnalytics::default();
    assert_eq!(analytics.super_chat_count, 0);
    assert_eq!(analytics.super_sticker_count, 0);
    assert_eq!(analytics.membership_gains, 0);
    assert!(analytics.hourly_stats.is_empty());
    assert!(analytics.top_contributors.is_empty());
    assert_eq!(analytics.super_chat_by_tier.total(), 0);
}

// ========================================================================
// SuperChatTierStats::increment - 全Tier個別検証 (07_revenue.md: Tier統計)
// 対象mutant: L49-53 の += が *= / -= に置換されるケースを検出
// ========================================================================

#[test]
fn tier_stats_increment_magenta() {
    // 07_revenue.md: Magenta tierのincrementが正しく加算されること
    let mut stats = SuperChatTierStats::default();
    stats.increment(SuperChatTier::Magenta);
    assert_eq!(stats.tier_magenta, 1);
}

#[test]
fn tier_stats_increment_orange() {
    // 07_revenue.md: Orange tierのincrementが正しく加算されること
    let mut stats = SuperChatTierStats::default();
    stats.increment(SuperChatTier::Orange);
    assert_eq!(stats.tier_orange, 1);
}

#[test]
fn tier_stats_increment_green() {
    // 07_revenue.md: Green tierのincrementが正しく加算されること
    let mut stats = SuperChatTierStats::default();
    stats.increment(SuperChatTier::Green);
    assert_eq!(stats.tier_green, 1);
}

#[test]
fn tier_stats_increment_cyan() {
    // 07_revenue.md: Cyan tierのincrementが正しく加算されること
    let mut stats = SuperChatTierStats::default();
    stats.increment(SuperChatTier::Cyan);
    assert_eq!(stats.tier_cyan, 1);
}

// ========================================================================
// SuperChatTierStats::total - 全加算項独立検証 (07_revenue.md: Tier統計)
// 対象mutant: L59-60 の + が - / * に置換されるケースを検出
// ========================================================================

#[test]
fn tier_stats_total_magenta_and_orange() {
    // 07_revenue.md: Magenta×1 + Orange×1 の合計が2になること（L59の加算パス検証）
    let mut stats = SuperChatTierStats::default();
    stats.increment(SuperChatTier::Magenta);
    stats.increment(SuperChatTier::Orange);
    assert_eq!(stats.total(), 2);
}

#[test]
fn tier_stats_total_green_and_cyan() {
    // 07_revenue.md: Green×1 + Cyan×1 の合計が2になること（L60の加算パス検証）
    let mut stats = SuperChatTierStats::default();
    stats.increment(SuperChatTier::Green);
    stats.increment(SuperChatTier::Cyan);
    assert_eq!(stats.total(), 2);
}

// ========================================================================
// calculate_session_statistics (07_revenue.md: セッション統計集計)
// 対象mutant:
//   L615: delete match arm "superchat" → super_chat_count がインクリメントされない
//   L616: super_chat_count += 1 → -= 1 / *= 1
//   L622: membership_count += 1 → -= 1 / *= 1
// ========================================================================

fn make_export_message(
    id: &str,
    author_id: &str,
    message_type: &str,
    tier: Option<SuperChatTier>,
) -> ExportMessage {
    ExportMessage {
        id: id.to_string(),
        timestamp: "2025-01-14T14:00:00Z".to_string(),
        author: "TestUser".to_string(),
        author_id: author_id.to_string(),
        content: "test content".to_string(),
        message_type: message_type.to_string(),
        amount_display: None,
        tier,
        is_moderator: Some(false),
        is_member: false,
        is_verified: Some(false),
        badges: Some(vec![]),
    }
}

#[test]
fn session_stats_super_chat_count_increments() {
    // 07_revenue.md: "superchat" メッセージが super_chat_count を1増やすこと
    // L615 (match arm削除mutant) と L616 (+= 1 → -= 1 / *= 1 mutant) を殺す
    let messages = vec![
        make_export_message("sc1", "UC_user1", "superchat", Some(SuperChatTier::Yellow)),
        make_export_message("sc2", "UC_user2", "superchat", Some(SuperChatTier::Red)),
        make_export_message("sc3", "UC_user3", "superchat", Some(SuperChatTier::Blue)),
    ];

    let stats = calculate_session_statistics(&messages, GiftStats::default());

    // 3件のsuperchatが正しく集計される
    assert_eq!(stats.super_chat_count, 3);
}

#[test]
fn session_stats_super_chat_count_not_incremented_for_non_superchat() {
    // 07_revenue.md: "chat" / "text" メッセージは super_chat_count に影響しないこと
    let messages = vec![
        make_export_message("msg1", "UC_user1", "text", None),
        make_export_message("msg2", "UC_user2", "text", None),
    ];

    let stats = calculate_session_statistics(&messages, GiftStats::default());

    assert_eq!(stats.super_chat_count, 0);
}

#[test]
fn session_stats_membership_count_increments() {
    // 07_revenue.md: "membership" メッセージが membership_count を1増やすこと
    // L622 (+= 1 → -= 1 / *= 1 mutant) を殺す
    let messages = vec![
        make_export_message("m1", "UC_user1", "membership", None),
        make_export_message("m2", "UC_user2", "membership", None),
    ];

    let stats = calculate_session_statistics(&messages, GiftStats::default());

    // 2件のmembershipが正しく集計される
    assert_eq!(stats.membership_count, 2);
}

#[test]
fn session_stats_membership_gift_count_increments() {
    // 07_revenue.md: "membership_gift" メッセージが membership_count を1増やすこと
    // L622 の "membership" | "membership_gift" パターン検証
    let messages = vec![
        make_export_message("mg1", "UC_user1", "membership_gift", None),
        make_export_message("mg2", "UC_user2", "membership_gift", None),
        make_export_message("mg3", "UC_user3", "membership_gift", None),
    ];

    let stats = calculate_session_statistics(&messages, GiftStats::default());

    assert_eq!(stats.membership_count, 3);
}

#[test]
fn session_stats_mixed_message_types() {
    // 07_revenue.md: superchat/membership/textが混在するとき各カウントが正しいこと
    // L615, L616, L622 の全mutantを同時に殺す
    let messages = vec![
        make_export_message("sc1", "UC_a", "superchat", Some(SuperChatTier::Red)),
        make_export_message("sc2", "UC_b", "superchat", Some(SuperChatTier::Yellow)),
        make_export_message("m1", "UC_c", "membership", None),
        make_export_message("t1", "UC_d", "text", None),
        make_export_message("t2", "UC_d", "text", None), // 同一ユーザーの重複
    ];

    let stats = calculate_session_statistics(&messages, GiftStats::default());

    assert_eq!(stats.super_chat_count, 2);
    assert_eq!(stats.membership_count, 1);
    assert_eq!(stats.total_messages, 5);
    assert_eq!(stats.unique_viewers, 4); // UC_dは1人
}

// ========================================================================
// SuperChatTier ordering (07_revenue.md: Blue < ... < Red)
// ========================================================================

#[test]
fn tier_ordering() {
    assert!(SuperChatTier::Blue < SuperChatTier::Cyan);
    assert!(SuperChatTier::Cyan < SuperChatTier::Green);
    assert!(SuperChatTier::Green < SuperChatTier::Yellow);
    assert!(SuperChatTier::Yellow < SuperChatTier::Orange);
    assert!(SuperChatTier::Orange < SuperChatTier::Magenta);
    assert!(SuperChatTier::Magenta < SuperChatTier::Red);
}

// ========================================================================
// sessions_analytics (07_revenue.md: 保存したメッセージから集計)
// ========================================================================

/// メッセージを 1 つのセッションに保存して、DB から分析する（07_revenue.md「集計の対象」）
///
/// make_chat_message は同じ視聴者に同じ id を付けるので、保存の重複排除に掛からないよう番号を足す
async fn analytics_of(messages: &[ChatMessage]) -> RevenueAnalytics {
    let numbered: Vec<ChatMessage> = messages
        .iter()
        .enumerate()
        .map(|(i, message)| ChatMessage {
            id: format!("{}#{}", message.id, i),
            ..message.clone()
        })
        .collect();
    let (db, session_id) = saved_session(&numbered).await;
    let conn = db.connection().await;
    sessions_analytics(&conn, &[session_id]).unwrap()
}

/// メッセージを 1 つのセッションに保存して、エクスポートの行として読む
async fn exports_of(messages: &[ChatMessage]) -> Vec<ExportMessage> {
    let (db, session_id) = saved_session(messages).await;
    let conn = db.connection().await;
    let config = ExportConfig {
        format: "json".to_string(),
        include_metadata: false,
        include_system_messages: true,
        max_records: None,
        sort_order: None,
    };
    export_messages(&conn, &[session_id], &config).unwrap().0
}

async fn saved_session(messages: &[ChatMessage]) -> (crate::database::Database, String) {
    let db = crate::database::Database::new_in_memory().unwrap();
    let session_id = {
        let conn = db.connection().await;
        let session_id = crate::database::create_session(&conn, None, None, None, None).unwrap();
        for message in messages {
            crate::database::save_message(&conn, &session_id, None, message, None).unwrap();
        }
        session_id
    };
    (db, session_id)
}

/// テスト用ChatMessageヘルパー
fn make_chat_message(
    channel_id: &str,
    author: &str,
    message_type: MessageType,
    metadata: Option<MessageMetadata>,
) -> ChatMessage {
    ChatMessage {
        id: format!("msg_{}", channel_id),
        channel_id: channel_id.to_string(),
        author: author.to_string(),
        message_type,
        metadata,
        ..Default::default()
    }
}

#[tokio::test]
async fn compute_revenue_analytics_empty_messages() {
    // 07_revenue.md: 空メッセージリスト → デフォルトのRevenueAnalytics
    let analytics = analytics_of(&[]).await;

    assert_eq!(analytics.super_chat_count, 0);
    assert_eq!(analytics.super_sticker_count, 0);
    assert_eq!(analytics.membership_gains, 0);
    assert_eq!(analytics.super_chat_by_tier.total(), 0);
    assert!(analytics.top_contributors.is_empty());
    assert!(analytics.hourly_stats.is_empty());
}

#[tokio::test]
async fn compute_revenue_analytics_mixed_types() {
    // 07_revenue.md: SuperChat×2 + SuperSticker×1 + Membership×1 → 正しい集計
    let messages = vec![
        make_chat_message(
            "UC_a",
            "UserA",
            MessageType::SuperChat {
                amount: "$10.00".to_string(),
            },
            Some(MessageMetadata {
                superchat_colors: Some(SuperChatColors {
                    header_background: "#ffb300".to_string(), // Yellow tier
                    header_text: "#000000".to_string(),
                    body_background: "#ffb300".to_string(),
                    body_text: "#000000".to_string(),
                }),
                amount: Some("$10.00".to_string()),
                badges: vec![],
                badge_info: vec![],
                color: None,
                is_moderator: false,
                is_verified: false,
            }),
        ),
        make_chat_message(
            "UC_b",
            "UserB",
            MessageType::SuperChat {
                amount: "$200.00".to_string(),
            },
            Some(MessageMetadata {
                superchat_colors: Some(SuperChatColors {
                    header_background: "#D00000".to_string(), // Red tier
                    header_text: "#ffffff".to_string(),
                    body_background: "#e62117".to_string(),
                    body_text: "#ffffff".to_string(),
                }),
                amount: Some("$200.00".to_string()),
                badges: vec![],
                badge_info: vec![],
                color: None,
                is_moderator: false,
                is_verified: false,
            }),
        ),
        make_chat_message(
            "UC_c",
            "UserC",
            MessageType::SuperSticker {
                amount: "$5.00".to_string(),
            },
            None,
        ),
        make_chat_message(
            "UC_d",
            "UserD",
            MessageType::Membership {
                milestone_months: Some(6),
            },
            None,
        ),
    ];

    let analytics = analytics_of(&messages).await;

    assert_eq!(analytics.super_chat_count, 2);
    assert_eq!(analytics.super_sticker_count, 1);
    assert_eq!(analytics.membership_gains, 1);
    assert_eq!(analytics.super_chat_by_tier.tier_yellow, 1);
    assert_eq!(analytics.super_chat_by_tier.tier_red, 1);
    assert_eq!(analytics.super_chat_by_tier.total(), 2);
    // 貢献者: UC_a(SC×1), UC_b(SC×1), UC_c(SS×1) = 3人
    assert_eq!(analytics.top_contributors.len(), 3);
}

#[tokio::test]
async fn compute_revenue_analytics_top_contributors_truncate() {
    // 07_revenue.md: 上位貢献者は10人にtruncateされる
    let messages: Vec<ChatMessage> = (0..15)
        .map(|i| {
            make_chat_message(
                &format!("UC_{}", i),
                &format!("User{}", i),
                MessageType::SuperChat {
                    amount: "$5.00".to_string(),
                },
                None,
            )
        })
        .collect();

    let analytics = analytics_of(&messages).await;

    assert_eq!(analytics.super_chat_count, 15);
    // 15人の貢献者がいるが上位10人にtruncateされる
    assert_eq!(analytics.top_contributors.len(), 10);
}

#[tokio::test]
async fn compute_revenue_analytics_contributors_sorted_by_count_then_tier() {
    // 07_revenue.md: SuperChat件数でソートし、同一件数の場合は最高tierで比較
    let messages = vec![
        // UC_a: SC×2, 最高tier=Red
        make_chat_message(
            "UC_a",
            "UserA",
            MessageType::SuperChat {
                amount: "$200.00".to_string(),
            },
            Some(MessageMetadata {
                superchat_colors: Some(SuperChatColors {
                    header_background: "#D00000".to_string(),
                    header_text: "#ffffff".to_string(),
                    body_background: "#e62117".to_string(),
                    body_text: "#ffffff".to_string(),
                }),
                amount: Some("$200.00".to_string()),
                badges: vec![],
                badge_info: vec![],
                color: None,
                is_moderator: false,
                is_verified: false,
            }),
        ),
        make_chat_message(
            "UC_a",
            "UserA",
            MessageType::SuperChat {
                amount: "$10.00".to_string(),
            },
            None,
        ),
        // UC_b: SC×2, 最高tier=Blue
        make_chat_message(
            "UC_b",
            "UserB",
            MessageType::SuperChat {
                amount: "$1.00".to_string(),
            },
            None,
        ),
        make_chat_message(
            "UC_b",
            "UserB",
            MessageType::SuperChat {
                amount: "$1.00".to_string(),
            },
            None,
        ),
        // UC_c: SC×1, 最高tier=Yellow
        make_chat_message(
            "UC_c",
            "UserC",
            MessageType::SuperChat {
                amount: "$15.00".to_string(),
            },
            None,
        ),
    ];

    let analytics = analytics_of(&messages).await;

    assert_eq!(analytics.top_contributors.len(), 3);
    // UC_a(2件, Red) と UC_b(2件, Blue) は同件数だがtierでUC_aが上
    assert_eq!(analytics.top_contributors[0].channel_id, "UC_a");
    assert_eq!(analytics.top_contributors[0].super_chat_count, 2);
    assert_eq!(analytics.top_contributors[1].channel_id, "UC_b");
    assert_eq!(analytics.top_contributors[1].super_chat_count, 2);
    // UC_c(1件) は最後
    assert_eq!(analytics.top_contributors[2].channel_id, "UC_c");
    assert_eq!(analytics.top_contributors[2].super_chat_count, 1);
}

#[tokio::test]
async fn compute_revenue_analytics_membership_gift_counted() {
    // 07_revenue.md: MembershipGiftもmembership_gainsにカウントされる
    let messages = vec![make_chat_message(
        "UC_a",
        "UserA",
        MessageType::MembershipGift { gift_count: 5 },
        None,
    )];

    let analytics = analytics_of(&messages).await;

    assert_eq!(analytics.membership_gains, 1);
}

#[tokio::test]
async fn compute_revenue_analytics_tier_escalation() {
    // 同一コントリビューターがBlue(低tier)→Red(高tier)の順で送信した場合、
    // 最高tierはRedに更新されること
    let messages = vec![
        // 1件目: Blue tier（メタデータなし = デフォルトBlue）
        make_chat_message(
            "UC_x",
            "UserX",
            MessageType::SuperChat {
                amount: "$1.00".to_string(),
            },
            None,
        ),
        // 2件目: Red tier
        make_chat_message(
            "UC_x",
            "UserX",
            MessageType::SuperChat {
                amount: "$200.00".to_string(),
            },
            Some(MessageMetadata {
                superchat_colors: Some(SuperChatColors {
                    header_background: "#D00000".to_string(),
                    header_text: "#ffffff".to_string(),
                    body_background: "#e62117".to_string(),
                    body_text: "#ffffff".to_string(),
                }),
                amount: Some("$200.00".to_string()),
                badges: vec![],
                badge_info: vec![],
                color: None,
                is_moderator: false,
                is_verified: false,
            }),
        ),
    ];

    let analytics = analytics_of(&messages).await;
    assert_eq!(analytics.top_contributors.len(), 1);
    assert_eq!(
        analytics.top_contributors[0].highest_tier,
        Some(SuperChatTier::Red)
    );
}

#[tokio::test]
async fn compute_revenue_analytics_supersticker_contributor_count() {
    // SuperStickerもcontributor件数にカウントされること
    let messages = vec![make_chat_message(
        "UC_s",
        "StickerUser",
        MessageType::SuperSticker {
            amount: "$5.00".to_string(),
        },
        None,
    )];

    let analytics = analytics_of(&messages).await;
    assert_eq!(analytics.top_contributors.len(), 1);
    assert_eq!(analytics.top_contributors[0].super_chat_count, 1);
}

// ========================================================================
// compute_session_analytics_from_rows (07_revenue.md: DB行データから集計)
// ========================================================================

#[test]
fn compute_session_analytics_from_rows_mixed() {
    // 07_revenue.md: 種類と色ごとの件数から集計する
    let rows = vec![
        ("superchat".to_string(), Some("#FFB300".to_string()), 1), // Yellow
        ("superchat".to_string(), Some("#D00000".to_string()), 1), // Red
        ("supersticker".to_string(), None, 1),
        ("membership".to_string(), None, 1),
        ("membership_gift".to_string(), None, 1),
        ("text".to_string(), None, 1),
    ];

    let analytics = compute_session_analytics_from_rows(&rows);

    assert_eq!(analytics.super_chat_count, 2);
    assert_eq!(analytics.super_sticker_count, 1);
    assert_eq!(analytics.membership_gains, 2); // membership + membership_gift
    assert_eq!(analytics.super_chat_by_tier.tier_yellow, 1);
    assert_eq!(analytics.super_chat_by_tier.tier_red, 1);
    assert_eq!(analytics.super_chat_by_tier.total(), 2);
    // DB行ベースの集計では貢献者トラッキングはしない
    assert!(analytics.top_contributors.is_empty());
}

#[test]
fn compute_session_analytics_from_rows_counts_each_row() {
    let rows = vec![("superchat".to_string(), Some("#00BFA5".to_string()), 3)];
    let analytics = compute_session_analytics_from_rows(&rows);
    assert_eq!(analytics.super_chat_count, 3);
    assert_eq!(analytics.super_chat_by_tier.tier_green, 3);
}

#[test]
fn compute_session_analytics_from_rows_empty() {
    // 07_revenue.md: 空の行リスト → デフォルトのRevenueAnalytics
    let analytics = compute_session_analytics_from_rows(&[]);

    assert_eq!(analytics.super_chat_count, 0);
    assert_eq!(analytics.super_sticker_count, 0);
    assert_eq!(analytics.membership_gains, 0);
    assert_eq!(analytics.super_chat_by_tier.total(), 0);
}

#[test]
fn compute_session_analytics_from_rows_superchat_without_color_is_unknown() {
    // 07_revenue.md: 色を保存する前の過去セッションのスーパーチャット → 段階不明（金額から推定しない）
    let rows = vec![("superchat".to_string(), None, 2)];

    let analytics = compute_session_analytics_from_rows(&rows);

    assert_eq!(analytics.super_chat_count, 2);
    assert_eq!(analytics.super_chat_by_tier.tier_unknown, 2);
    assert_eq!(analytics.super_chat_by_tier.tier_blue, 0);
}

// ========================================================================
// export_messages (07_revenue.md: 保存したメッセージ→ExportMessage)
// ========================================================================

#[tokio::test]
async fn convert_messages_to_export_text() {
    // 07_revenue.md: TextメッセージはExportMessageのmessage_type="text"に変換
    let messages = vec![ChatMessage {
        id: "msg1".to_string(),
        timestamp: "2025-01-14T14:00:00Z".to_string(),
        author: "TestUser".to_string(),
        channel_id: "UC_test".to_string(),
        content: "Hello".to_string(),
        message_type: MessageType::Text,
        is_member: false,
        ..Default::default()
    }];

    let exports = exports_of(&messages).await;

    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].message_type, "text");
    assert_eq!(exports[0].id, "msg1");
    assert_eq!(exports[0].author, "TestUser");
    assert_eq!(exports[0].author_id, "UC_test");
    assert_eq!(exports[0].content, "Hello");
    assert!(exports[0].amount_display.is_none());
    assert!(exports[0].tier.is_none());
    // metadata が無ければバッジは不明（08_database.md）
    assert_eq!(exports[0].is_moderator, None);
    assert!(!exports[0].is_member);
    assert_eq!(exports[0].is_verified, None);
}

#[tokio::test]
async fn convert_messages_to_export_superchat_with_color() {
    // 07_revenue.md: SuperChatは色情報からtierを判定し、amountとtierを含む
    let messages = vec![ChatMessage {
        id: "sc1".to_string(),
        timestamp: "2025-01-14T14:01:00Z".to_string(),
        author: "SCUser".to_string(),
        channel_id: "UC_sc".to_string(),
        content: "Super!".to_string(),
        message_type: MessageType::SuperChat {
            amount: "$50.00".to_string(),
        },
        metadata: Some(MessageMetadata {
            superchat_colors: Some(SuperChatColors {
                header_background: "#C2185B".to_string(), // Magenta
                header_text: "#ffffff".to_string(),
                body_background: "#e91e63".to_string(),
                body_text: "#ffffff".to_string(),
            }),
            amount: Some("$50.00".to_string()),
            badges: vec!["member".to_string()],
            badge_info: vec![],
            color: None,
            is_moderator: true,
            is_verified: false,
        }),
        is_member: true,
        ..Default::default()
    }];

    let exports = exports_of(&messages).await;

    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].message_type, "superchat");
    assert_eq!(exports[0].amount_display, Some("$50.00".to_string()));
    assert_eq!(exports[0].tier, Some(SuperChatTier::Magenta));
    assert_eq!(exports[0].is_moderator, Some(true));
    assert!(exports[0].is_member);
    assert_eq!(exports[0].badges, Some(vec!["member".to_string()]));
}

#[tokio::test]
async fn convert_messages_to_export_supersticker() {
    // 07_revenue.md: SuperStickerはtierなし、amountあり
    let messages = vec![ChatMessage {
        id: "ss1".to_string(),
        message_type: MessageType::SuperSticker {
            amount: "$5.00".to_string(),
        },
        ..Default::default()
    }];

    let exports = exports_of(&messages).await;

    assert_eq!(exports[0].message_type, "supersticker");
    assert_eq!(exports[0].amount_display, Some("$5.00".to_string()));
    assert!(exports[0].tier.is_none());
}

#[tokio::test]
async fn convert_messages_to_export_membership() {
    // 07_revenue.md: Membershipはmessage_type="membership"
    let messages = vec![ChatMessage {
        id: "m1".to_string(),
        message_type: MessageType::Membership {
            milestone_months: Some(12),
        },
        ..Default::default()
    }];

    let exports = exports_of(&messages).await;

    assert_eq!(exports[0].message_type, "membership");
    assert!(exports[0].amount_display.is_none());
    assert!(exports[0].tier.is_none());
}

#[tokio::test]
async fn convert_messages_to_export_membership_gift() {
    // 07_revenue.md: MembershipGiftはmessage_type="membership_gift"
    let messages = vec![ChatMessage {
        id: "mg1".to_string(),
        message_type: MessageType::MembershipGift { gift_count: 5 },
        ..Default::default()
    }];

    let exports = exports_of(&messages).await;

    assert_eq!(exports[0].message_type, "membership_gift");
}

#[tokio::test]
async fn convert_messages_to_export_system() {
    // 07_revenue.md: Systemはmessage_type="system"
    let messages = vec![ChatMessage {
        id: "sys1".to_string(),
        message_type: MessageType::System,
        ..Default::default()
    }];

    let exports = exports_of(&messages).await;

    assert_eq!(exports[0].message_type, "system");
    assert!(exports[0].amount_display.is_none());
    assert!(exports[0].tier.is_none());
}

#[tokio::test]
async fn convert_messages_to_export_all_types() {
    // 07_revenue.md: 全MessageTypeを含むリストが正しく変換される
    let messages = vec![
        ChatMessage {
            id: "1".to_string(),
            message_type: MessageType::Text,
            ..Default::default()
        },
        ChatMessage {
            id: "2".to_string(),
            message_type: MessageType::SuperChat {
                amount: "$10.00".to_string(),
            },
            ..Default::default()
        },
        ChatMessage {
            id: "3".to_string(),
            message_type: MessageType::SuperSticker {
                amount: "$3.00".to_string(),
            },
            ..Default::default()
        },
        ChatMessage {
            id: "4".to_string(),
            message_type: MessageType::Membership {
                milestone_months: None,
            },
            ..Default::default()
        },
        ChatMessage {
            id: "5".to_string(),
            message_type: MessageType::MembershipGift { gift_count: 1 },
            ..Default::default()
        },
        ChatMessage {
            id: "6".to_string(),
            message_type: MessageType::System,
            ..Default::default()
        },
    ];

    let exports = exports_of(&messages).await;

    assert_eq!(exports.len(), 6);
    assert_eq!(exports[0].message_type, "text");
    assert_eq!(exports[1].message_type, "superchat");
    assert_eq!(exports[2].message_type, "supersticker");
    assert_eq!(exports[3].message_type, "membership");
    assert_eq!(exports[4].message_type, "membership_gift");
    assert_eq!(exports[5].message_type, "system");
}

// ========================================================================
// 追加テスト: 残存missed mutantsを殺す
// ========================================================================

#[tokio::test]
async fn compute_revenue_analytics_supersticker_multiple_count() {
    // 07_revenue.md: 同一コントリビューターが複数SuperStickerを送信した場合、
    // contributor件数が正しく加算されること
    // 対象mutant: L278 `entry.1 += 1` → `*= 1`
    // (*= 1 の場合、2件目以降で count が 1×1=1 のままになる)
    let messages = vec![
        make_chat_message(
            "UC_s",
            "StickerUser",
            MessageType::SuperSticker {
                amount: "$5.00".to_string(),
            },
            None,
        ),
        make_chat_message(
            "UC_s",
            "StickerUser",
            MessageType::SuperSticker {
                amount: "$5.00".to_string(),
            },
            None,
        ),
        make_chat_message(
            "UC_s",
            "StickerUser",
            MessageType::SuperSticker {
                amount: "$5.00".to_string(),
            },
            None,
        ),
    ];

    let analytics = analytics_of(&messages).await;

    // 3件のSuperStickerで件数は3になるべき (*= 1 mutantでは1になる)
    assert_eq!(analytics.top_contributors.len(), 1);
    assert_eq!(analytics.top_contributors[0].super_chat_count, 3);
}

#[test]
fn export_to_csv_without_metadata_data_rows_present() {
    // 07_revenue.md: include_metadata=false のCSVにはメタデータヘッダなし、
    // データ行は正しく出力されること
    // 対象mutant: export_to_csv内のinclude_metadataブランチ
    let data = make_test_export_data();
    let config = ExportConfig {
        format: "csv".to_string(),
        include_metadata: false,
        include_system_messages: false,
        max_records: None,
        sort_order: None,
    };

    let csv = export_to_csv(&data, &config).unwrap();

    // メタデータヘッダは含まれない
    assert!(!csv.contains("# Metadata"));
    assert!(!csv.contains("# Session ID"));
    // カラムヘッダから始まる
    assert!(csv.starts_with("id,timestamp,author"));
    // データ行が含まれる (msg1, msg2 の両方)
    assert!(csv.contains("\"msg1\""));
    assert!(csv.contains("\"msg2\""));
    // SuperChatのtier情報が含まれる
    assert!(csv.contains("yellow"));
    assert!(csv.contains("$10.00"));
}

#[test]
fn export_to_json_messages_only_content_verified() {
    // 07_revenue.md: include_metadata=false のJSONはmessagesの配列のみ返し、
    // 各要素のフィールドが正しいこと
    // 対象mutant: export_to_json内のinclude_metadataブランチ
    let data = make_test_export_data();
    let config = ExportConfig {
        format: "json".to_string(),
        include_metadata: false,
        include_system_messages: false,
        max_records: None,
        sort_order: None,
    };

    let json = export_to_json(&data, &config).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // トップレベルは配列
    assert!(parsed.is_array());
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    // metadata/statistics フィールドは含まれない (配列なので存在しない)
    assert!(parsed.get("metadata").is_none());
    assert!(parsed.get("statistics").is_none());

    // 各メッセージのフィールドを検証
    let first = &arr[0];
    assert_eq!(first["id"], "msg1");
    assert_eq!(first["message_type"], "text");

    let second = &arr[1];
    assert_eq!(second["id"], "msg2");
    assert_eq!(second["message_type"], "superchat");
    assert_eq!(second["amount_display"], "$10.00");
}

// ========================================================================
// ギフト集計 (07_revenue.md: ギフト集計)
// ========================================================================

fn gift(name: &str, jewels: Option<u32>) -> GiftDetails {
    GiftDetails {
        gift_name: name.to_string(),
        gift_image_url: Some(format!("https://example.com/{}.png", name)),
        jewel_count: jewels,
    }
}

#[test]
fn gift_stats_empty() {
    let stats = GiftStats::from_gifts(&[] as &[GiftDetails]);
    assert_eq!(stats.gift_count, 0);
    assert!(stats.gifts_by_name.is_empty());
    assert_eq!(stats.jewel_known_count, 0);
    assert_eq!(stats.total_jewels, 0);
}

#[test]
fn gift_stats_spec_example() {
    // Hiding(不明)・Press F(10)・Press F(10) → 3件、Press F 2件・Hiding 1件、判明 2件、合計 20
    let stats = GiftStats::from_gifts(&[
        gift("Hiding", None),
        gift("Press F", Some(10)),
        gift("Press F", Some(10)),
    ]);
    assert_eq!(stats.gift_count, 3);
    let by_name: Vec<(&str, usize)> = stats
        .gifts_by_name
        .iter()
        .map(|g| (g.gift_name.as_str(), g.count))
        .collect();
    assert_eq!(by_name, vec![("Press F", 2), ("Hiding", 1)]);
    assert_eq!(
        stats.gifts_by_name[0].gift_image_url.as_deref(),
        Some("https://example.com/Press F.png")
    );
    assert_eq!(stats.jewel_known_count, 2);
    assert_eq!(stats.total_jewels, 20);
}

#[test]
fn gift_stats_same_count_sorted_by_name() {
    let stats = GiftStats::from_gifts(&[gift("Zebra", None), gift("Apple", None)]);
    let names: Vec<&str> = stats
        .gifts_by_name
        .iter()
        .map(|g| g.gift_name.as_str())
        .collect();
    assert_eq!(names, vec!["Apple", "Zebra"]);
}

#[tokio::test]
async fn compute_revenue_analytics_counts_gifts_separately() {
    let messages = vec![
        make_chat_message("", "@a", MessageType::Gift(gift("Hiding", None)), None),
        make_chat_message(
            "UC_b",
            "@b",
            MessageType::Gift(gift("Press F", Some(10))),
            None,
        ),
        make_chat_message(
            "UC_c",
            "@c",
            MessageType::SuperChat {
                amount: "$10.00".to_string(),
            },
            None,
        ),
    ];
    let analytics = analytics_of(&messages).await;
    assert_eq!(analytics.gifts.gift_count, 2);
    assert_eq!(analytics.gifts.total_jewels, 10);
    // 既存の集計には影響しない
    assert_eq!(analytics.super_chat_count, 1);
    assert_eq!(analytics.membership_gains, 0);
    assert_eq!(analytics.top_contributors.len(), 1);
}

#[test]
fn gift_stats_from_db_metadata() {
    // 08_database.md: gift 行の metadata JSON から集計する。壊れた JSON は数えない
    let rows = vec![
        Some(r#"{"gift_name":"Press F","gift_image_url":null,"jewel_count":10}"#.to_string()),
        Some(r#"{"gift_name":"Hiding","gift_image_url":null,"jewel_count":null}"#.to_string()),
        Some("not json".to_string()),
        None,
    ];
    let stats = gift_stats_from_metadata(&rows);
    assert_eq!(stats.gift_count, 2);
    assert_eq!(stats.jewel_known_count, 1);
    assert_eq!(stats.total_jewels, 10);
}

#[tokio::test]
async fn convert_messages_to_export_gift() {
    // 07_revenue.md: message_type = gift、amount_display = "10 Jewels"（不明なら空）
    let messages = vec![
        make_chat_message(
            "UC_b",
            "@b",
            MessageType::Gift(gift("Press F", Some(10))),
            None,
        ),
        make_chat_message("", "@a", MessageType::Gift(gift("Hiding", None)), None),
    ];
    let exported = exports_of(&messages).await;
    assert_eq!(exported[0].message_type, "gift");
    assert_eq!(exported[0].amount_display.as_deref(), Some("10 Jewels"));
    assert_eq!(exported[1].message_type, "gift");
    assert_eq!(exported[1].amount_display, None);
}

#[test]
fn json_export_statistics_include_gifts() {
    let mut data = make_test_export_data();
    data.statistics.gifts = GiftStats::from_gifts(&[gift("Press F", Some(10))]);
    let config = ExportConfig {
        format: "json".to_string(),
        include_metadata: true,
        include_system_messages: true,
        max_records: None,
        sort_order: None,
    };
    let json: serde_json::Value =
        serde_json::from_str(&export_to_json(&data, &config).unwrap()).unwrap();
    assert_eq!(json["statistics"]["gifts"]["gift_count"], 1);
    assert_eq!(json["statistics"]["gifts"]["total_jewels"], 10);
}

// ========================================================================
// 過去セッションの分析・エクスポートを実際の DB で通す
// (07_revenue.md / 08_database.md。SQL の列名の誤りは純粋関数のテストでは見つからない)
// ========================================================================

mod with_db {
    use super::*;
    use crate::core::{BadgeInfo, MessageMetadata, SuperChatColors};
    use crate::database::{self, Database};

    fn superchat(id: &str, header: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            timestamp: "2026-09-28T12:00:00Z".to_string(),
            timestamp_usec: "1790000000000000".to_string(),
            author: "@donor".to_string(),
            channel_id: "UC_donor".to_string(),
            content: "応援してます".to_string(),
            message_type: MessageType::SuperChat {
                amount: "¥500".to_string(),
            },
            metadata: Some(MessageMetadata {
                amount: Some("¥500".to_string()),
                superchat_colors: Some(SuperChatColors {
                    header_background: header.to_string(),
                    header_text: "#FFFFFF".to_string(),
                    body_background: header.to_string(),
                    body_text: "#FFFFFF".to_string(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn moderator_text(id: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            timestamp: "2026-09-28T12:00:01Z".to_string(),
            timestamp_usec: "1790000001000000".to_string(),
            author: "@mod".to_string(),
            channel_id: "UC_mod".to_string(),
            content: "モデレーターです".to_string(),
            message_type: MessageType::Text,
            metadata: Some(MessageMetadata {
                badges: vec!["moderator".to_string()],
                badge_info: vec![BadgeInfo {
                    badge_type: "moderator".to_string(),
                    label: "Moderator".to_string(),
                    tooltip: Some("Moderator".to_string()),
                    icon_url: None,
                }],
                is_moderator: true,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn gift(id: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            timestamp: "2026-09-28T12:00:02Z".to_string(),
            timestamp_usec: "1790000002000000".to_string(),
            author: "@gifter".to_string(),
            content: "Press F".to_string(),
            message_type: MessageType::Gift(GiftDetails {
                gift_name: "Press F".to_string(),
                gift_image_url: None,
                jewel_count: Some(10),
            }),
            ..Default::default()
        }
    }

    /// 005 より前に保存した行を再現する（色とバッジを NULL に戻す）
    fn forget_columns(conn: &rusqlite::Connection, message_id: &str) {
        conn.execute(
            "UPDATE messages SET superchat_color = NULL, is_moderator = NULL,
                        is_verified = NULL, badges = NULL WHERE message_id = ?1",
            [message_id],
        )
        .unwrap();
    }

    async fn session_with(messages: &[ChatMessage], old: &[&str]) -> (Database, String) {
        let db = Database::new_in_memory().unwrap();
        let session_id = {
            let conn = db.connection().await;
            let session_id =
                database::create_session(&conn, None, Some("配信"), None, None).unwrap();
            for message in messages {
                database::save_message(&conn, &session_id, None, message, None).unwrap();
            }
            for id in old {
                forget_columns(&conn, id);
            }
            session_id
        };
        (db, session_id)
    }

    fn export_config() -> ExportConfig {
        ExportConfig {
            format: "json".to_string(),
            include_metadata: true,
            include_system_messages: true,
            max_records: None,
            sort_order: None,
        }
    }

    #[tokio::test]
    async fn session_analytics_counts_saved_color_and_old_rows_as_unknown() {
        // 色を保存した緑のスパチャ 1 件 + 色を保存する前のスパチャ 1 件
        let messages = [
            superchat("sc_new", "#00BFA5"),
            superchat("sc_old", "#00BFA5"),
            gift("g1"),
        ];
        let (db, session_id) = session_with(&messages, &["sc_old"]).await;
        let conn = db.connection().await;

        let analytics = sessions_analytics(&conn, std::slice::from_ref(&session_id)).unwrap();

        assert_eq!(analytics.super_chat_count, 2);
        assert_eq!(analytics.super_chat_by_tier.tier_green, 1);
        assert_eq!(analytics.super_chat_by_tier.tier_unknown, 1);
        assert_eq!(analytics.gifts.gift_count, 1);
    }

    #[tokio::test]
    async fn session_export_reads_saved_badges_and_leaves_unknown_as_none() {
        let messages = [
            moderator_text("mod_new"),
            moderator_text("mod_old"),
            superchat("sc1", "#C2185B"),
            gift("g1"),
        ];
        let (db, session_id) = session_with(&messages, &["mod_old"]).await;
        let conn = db.connection().await;

        let data = session_export_data(&conn, &session_id, &export_config()).unwrap();
        let by_id = |id: &str| data.messages.iter().find(|m| m.id == id).unwrap();

        // id は YouTube のメッセージ ID
        assert_eq!(data.messages.len(), 4);
        let saved = by_id("mod_new");
        assert_eq!(saved.is_moderator, Some(true));
        assert_eq!(saved.is_verified, Some(false));
        assert_eq!(saved.badges, Some(vec!["moderator".to_string()]));
        // 005 より前の行は不明
        let old = by_id("mod_old");
        assert_eq!(old.is_moderator, None);
        assert_eq!(old.badges, None);
        // バッジを読めない種類（ギフト）は不明
        assert_eq!(by_id("g1").is_moderator, None);
        assert_eq!(by_id("g1").amount_display.as_deref(), Some("10 Jewels"));
        // 段階は保存した色から
        assert_eq!(by_id("sc1").tier, Some(SuperChatTier::Magenta));
    }

    #[tokio::test]
    async fn session_export_csv_leaves_unknown_blank() {
        let (db, session_id) = session_with(&[moderator_text("mod_old")], &["mod_old"]).await;
        let conn = db.connection().await;
        let config = ExportConfig {
            format: "csv".to_string(),
            include_metadata: false,
            ..export_config()
        };

        let data = session_export_data(&conn, &session_id, &config).unwrap();
        let csv = export_to_csv(&data, &config).unwrap();
        let row = csv.lines().nth(1).unwrap();

        // ...,tier,is_moderator,is_member,is_verified,badges
        assert!(row.ends_with(r#","",,false,,"""#), "{row}");
    }

    /// 開始時刻を指定してセッションを作り、SuperChat を `count` 件保存する
    fn session_at(
        conn: &rusqlite::Connection,
        start_time: &str,
        prefix: &str,
        count: usize,
    ) -> String {
        let session_id = database::create_session(conn, None, None, None, None).unwrap();
        conn.execute(
            "UPDATE sessions SET start_time = ?1 WHERE id = ?2",
            [start_time, &session_id],
        )
        .unwrap();
        for i in 0..count {
            let message = superchat(&format!("{prefix}{i}"), "#1565C0");
            database::save_message(conn, &session_id, None, &message, None).unwrap();
        }
        session_id
    }

    // 07_revenue.md「集計の対象」: 配信 A → 切断 → 配信 B なら A と B の合計。前回の起動のセッションは数えない
    #[tokio::test]
    async fn current_scope_is_sessions_started_since_launch() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.connection().await;
        let launched_at = DateTime::parse_from_rfc3339("2026-10-03T10:00:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        session_at(&conn, "2026-10-02T20:00:00+00:00", "prev", 5);
        let a = session_at(&conn, "2026-10-03T10:05:00+00:00", "a", 2);
        let b = session_at(&conn, "2026-10-03T11:00:00.123456789+00:00", "b", 3);

        let session_ids = sessions_started_since(&conn, launched_at).unwrap();
        assert_eq!(session_ids, [a, b]);
        let analytics = sessions_analytics(&conn, &session_ids).unwrap();
        assert_eq!(analytics.super_chat_count, 5);
    }

    // 07_revenue.md「集計の対象」: メモリ上のバッファの上限（1000件）に依らない
    #[tokio::test]
    async fn current_scope_counts_beyond_message_buffer() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.connection().await;
        let session_id = session_at(&conn, "2026-10-03T10:05:00+00:00", "sc", 1500);
        let config = ExportConfig {
            max_records: None,
            ..export_config()
        };

        let analytics = sessions_analytics(&conn, std::slice::from_ref(&session_id)).unwrap();
        let (messages, statistics) = export_messages(&conn, &[session_id], &config).unwrap();

        assert_eq!(analytics.super_chat_count, 1500);
        assert_eq!(messages.len(), 1500);
        assert_eq!(statistics.super_chat_count, 1500);
    }

    // 07_revenue.md「上位貢献者」: 件数は SuperChat と SuperSticker の合計、表示名は最初のときの名前
    #[tokio::test]
    async fn top_contributors_use_first_name_and_count_stickers() {
        let renamed = ChatMessage {
            id: "sc2".to_string(),
            author: "@donor-renamed".to_string(),
            ..superchat("sc2", "#D00000")
        };
        let sticker = ChatMessage {
            id: "st1".to_string(),
            message_type: MessageType::SuperSticker {
                amount: "¥200".to_string(),
            },
            metadata: None,
            ..superchat("st1", "#00B8D4")
        };
        let (db, session_id) =
            session_with(&[superchat("sc1", "#1565C0"), renamed, sticker], &[]).await;
        let conn = db.connection().await;

        let analytics = sessions_analytics(&conn, &[session_id]).unwrap();

        assert_eq!(analytics.top_contributors.len(), 1);
        let top = &analytics.top_contributors[0];
        assert_eq!(top.display_name, "@donor");
        assert_eq!(top.super_chat_count, 3);
        assert_eq!(top.highest_tier, Some(SuperChatTier::Red));
    }
}
