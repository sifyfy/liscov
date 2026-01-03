use serde::{Deserialize, Serialize};
use tracing::debug;

/// GUI用のチャットメッセージ構造体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GuiChatMessage {
    pub id: String,                      // メッセージの一意識別子
    pub timestamp: String,               // 表示用タイムスタンプ (HH:MM:SS)
    pub timestamp_usec: String,          // オリジナルタイムスタンプ (マイクロ秒、ソート用)
    pub message_type: MessageType,
    pub author: String,
    pub author_icon_url: Option<String>, // 投稿者のアイコンURL
    pub channel_id: String,
    pub content: String,
    pub runs: Vec<MessageRun>, // テキストとスタンプを分離したparts
    pub metadata: Option<MessageMetadata>,
    pub is_member: bool,            // メンバーかどうかの判定フラグ
    pub comment_count: Option<u32>, // この配信での投稿者のコメント回数
}

impl GuiChatMessage {
    /// テスト用にIDとタイムスタンプを生成してメッセージを作成
    #[cfg(test)]
    pub fn new_for_test(
        author: &str,
        content: &str,
        message_type: MessageType,
    ) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);
        let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);

        Self {
            id: format!("test_{}", counter),
            timestamp: "00:00:00".to_string(),
            timestamp_usec: counter.to_string(),
            message_type,
            author: author.to_string(),
            content: content.to_string(),
            ..Default::default()
        }
    }

    /// テスト用にIDとタイムスタンプを自動生成（既存のフィールド値を保持）
    #[cfg(test)]
    pub fn with_test_id(mut self) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);
        let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);

        if self.id.is_empty() {
            self.id = format!("test_{}", counter);
        }
        if self.timestamp.is_empty() {
            self.timestamp = "00:00:00".to_string();
        }
        if self.timestamp_usec.is_empty() {
            self.timestamp_usec = counter.to_string();
        }
        self
    }
}

/// メッセージの一部（テキストまたはスタンプ）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRun {
    Text {
        content: String,
    },
    Emoji {
        emoji_id: String,
        image_url: String,
        alt_text: String,
    },
}

impl Default for MessageRun {
    fn default() -> Self {
        MessageRun::Text {
            content: String::new(),
        }
    }
}

/// メッセージタイプ列挙型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum MessageType {
    #[default]
    Text,
    SuperChat {
        amount: String,
    },
    SuperSticker {
        amount: String,
    },
    /// メンバーシップ関連メッセージ
    /// - milestone_months: None = 新規メンバー加入
    /// - milestone_months: Some(n) = n ヶ月継続のマイルストーン
    Membership {
        milestone_months: Option<u32>,
    },
    /// メンバーシップギフト
    /// - gift_count: ギフトしたメンバーシップの数
    MembershipGift {
        gift_count: u32,
    },
    System,
}

impl MessageType {
    pub fn as_string(&self) -> String {
        match self {
            MessageType::Text => "text".to_string(),
            MessageType::SuperChat { .. } => "super-chat".to_string(),
            MessageType::SuperSticker { .. } => "super-sticker".to_string(),
            MessageType::Membership { milestone_months } => {
                if milestone_months.is_some() {
                    "membership-milestone".to_string()
                } else {
                    "membership".to_string()
                }
            }
            MessageType::MembershipGift { .. } => "membership-gift".to_string(),
            MessageType::System => "system".to_string(),
        }
    }
}

/// バッジ情報
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BadgeInfo {
    pub tooltip: String,
    pub image_url: Option<String>, // バッジ画像URL
}

/// スーパーチャット/スーパーステッカーの色情報（YouTubeから取得）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SuperChatColors {
    /// ヘッダー背景色 (hex形式: #RRGGBB)
    pub header_background: String,
    /// ヘッダーテキスト色
    pub header_text: String,
    /// ボディ背景色
    pub body_background: String,
    /// ボディテキスト色
    pub body_text: String,
}

/// 金額文字列から日本語の通貨名を取得
/// 日本円の場合はNoneを返す（表示不要のため）
pub fn get_currency_name_ja(amount: &str) -> Option<&'static str> {
    let amount = amount.trim();

    // 日本円は表示不要
    if amount.starts_with('¥') || amount.starts_with('￥') || amount.ends_with("JPY") {
        return None;
    }

    // プレフィックス付き通貨（より具体的なものを先にチェック）
    if amount.starts_with("CA$") || amount.starts_with("C$") {
        return Some("カナダドル");
    }
    if amount.starts_with("A$") || amount.starts_with("AU$") || amount.starts_with("AUD") {
        return Some("豪ドル");
    }
    if amount.starts_with("HK$") || amount.starts_with("HKD") {
        return Some("香港ドル");
    }
    if amount.starts_with("NT$") || amount.starts_with("NTD") || amount.starts_with("TWD") {
        return Some("台湾ドル");
    }
    if amount.starts_with("S$") || amount.starts_with("SGD") {
        return Some("シンガポールドル");
    }
    if amount.starts_with("NZ$") || amount.starts_with("NZD") {
        return Some("NZドル");
    }
    if amount.starts_with("MX$") || amount.starts_with("MXN") {
        return Some("メキシコペソ");
    }
    if amount.starts_with("R$") || amount.starts_with("BRL") {
        return Some("ブラジルレアル");
    }

    // 単一記号通貨
    if amount.starts_with('$') || amount.starts_with("USD") {
        return Some("米ドル");
    }
    if amount.starts_with('€') || amount.starts_with("EUR") {
        return Some("ユーロ");
    }
    if amount.starts_with('£') || amount.starts_with("GBP") {
        return Some("英ポンド");
    }
    if amount.starts_with('₩') || amount.starts_with("KRW") {
        return Some("韓国ウォン");
    }
    if amount.starts_with('₹') || amount.starts_with("INR") {
        return Some("インドルピー");
    }
    if amount.starts_with('₱') || amount.starts_with("PHP") {
        return Some("フィリピンペソ");
    }
    if amount.starts_with('฿') || amount.starts_with("THB") {
        return Some("タイバーツ");
    }
    if amount.starts_with("RM") || amount.starts_with("MYR") {
        return Some("マレーシアリンギット");
    }
    if amount.starts_with("Rp") || amount.starts_with("IDR") {
        return Some("インドネシアルピア");
    }
    if amount.starts_with("CHF") {
        return Some("スイスフラン");
    }
    if amount.starts_with("SEK") {
        return Some("スウェーデンクローナ");
    }
    if amount.starts_with("NOK") {
        return Some("ノルウェークローネ");
    }
    if amount.starts_with("DKK") {
        return Some("デンマーククローネ");
    }
    if amount.starts_with("PLN") || amount.starts_with("zł") {
        return Some("ポーランドズロチ");
    }
    if amount.starts_with("CZK") || amount.starts_with("Kč") {
        return Some("チェココルナ");
    }
    if amount.starts_with("HUF") || amount.starts_with("Ft") {
        return Some("ハンガリーフォリント");
    }
    if amount.starts_with("RUB") || amount.starts_with('₽') {
        return Some("ロシアルーブル");
    }
    if amount.starts_with("TRY") || amount.starts_with('₺') {
        return Some("トルコリラ");
    }
    if amount.starts_with("ZAR") {
        return Some("南アフリカランド");
    }
    if amount.starts_with("ARS") {
        return Some("アルゼンチンペソ");
    }
    if amount.starts_with("CLP") {
        return Some("チリペソ");
    }
    if amount.starts_with("COP") {
        return Some("コロンビアペソ");
    }
    if amount.starts_with("PEN") {
        return Some("ペルーソル");
    }
    if amount.starts_with("VND") || amount.starts_with('₫') {
        return Some("ベトナムドン");
    }
    if amount.starts_with("EGP") {
        return Some("エジプトポンド");
    }
    if amount.starts_with("SAR") {
        return Some("サウジアラビアリヤル");
    }
    if amount.starts_with("AED") {
        return Some("UAEディルハム");
    }
    if amount.starts_with("ILS") || amount.starts_with('₪') {
        return Some("イスラエルシェケル");
    }

    // 不明な通貨（日本円以外で認識できない通貨）
    Some("不明な外貨")
}

/// メッセージメタデータ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MessageMetadata {
    pub amount: Option<String>,
    pub badges: Vec<String>,        // 後方互換性のため残す
    pub badge_info: Vec<BadgeInfo>, // 新しいバッジ情報
    pub color: Option<String>,
    pub is_moderator: bool, // モデレーターかどうか
    pub is_verified: bool,  // 認証済みかどうか
    /// スーパーチャット/スーパーステッカーの色情報
    pub superchat_colors: Option<SuperChatColors>,
}

/// 既存のliscovライブラリからGUI用メッセージへの変換
impl From<crate::get_live_chat::ChatItem> for GuiChatMessage {
    fn from(item: crate::get_live_chat::ChatItem) -> Self {
        match item {
            crate::get_live_chat::ChatItem::TextMessage { renderer } => {
                // 新しい構造：runsを分離して管理
                let mut runs = Vec::new();
                let mut content_parts = Vec::new();

                for run in &renderer.message.runs {
                    if let Some(text) = run.get_text() {
                        runs.push(MessageRun::Text {
                            content: text.to_string(),
                        });
                        content_parts.push(text.to_string());
                    } else if let Some(emoji) = run.get_emoji() {
                        let image_url = emoji
                            .image
                            .thumbnails
                            .first()
                            .map(|t| t.url.clone())
                            .unwrap_or_default();

                        let alt_text = if let Some(accessibility) = &emoji.image.accessibility {
                            accessibility.accessibility_data.label.clone()
                        } else {
                            format!("Emoji: {}", emoji.emoji_id)
                        };

                        runs.push(MessageRun::Emoji {
                            emoji_id: emoji.emoji_id.clone(),
                            image_url,
                            alt_text: alt_text.clone(),
                        });

                        // contentにはalt_textを入れる（検索・フィルタリング用）
                        content_parts.push(alt_text);
                    }
                }

                let (badges, badge_info, is_member, is_moderator, is_verified) =
                    extract_badge_info(&renderer.author_badges);

                // アイコンURL抽出
                let author_icon_url = renderer
                    .author_photo
                    .thumbnails
                    .first()
                    .map(|thumbnail| thumbnail.url.clone());

                // タイムスタンプ変換（マイクロ秒 → 表示用）
                let display_timestamp = timestamp_usec_to_display(&renderer.timestamp_usec);

                Self {
                    id: renderer.id.clone(),
                    timestamp: display_timestamp,
                    timestamp_usec: renderer.timestamp_usec.clone(),
                    message_type: MessageType::Text,
                    author: renderer.author_name.simple_text.clone(),
                    author_icon_url,
                    channel_id: renderer.author_external_channel_id.clone(),
                    content: content_parts.join(""),
                    runs,
                    metadata: Some(MessageMetadata {
                        amount: None,
                        badges,
                        badge_info,
                        color: None,
                        is_moderator,
                        is_verified,
                        superchat_colors: None,
                    }),
                    is_member,
                    comment_count: None, // StateManagerで後から設定される
                }
            }
            crate::get_live_chat::ChatItem::PaidMessage { renderer } => {
                let (badges, badge_info, is_member, is_moderator, is_verified) =
                    extract_badge_info(&renderer.author_badges);

                // アイコンURL抽出
                let author_icon_url = renderer
                    .author_photo
                    .thumbnails
                    .first()
                    .map(|thumbnail| thumbnail.url.clone());

                // タイムスタンプ変換（マイクロ秒 → 表示用）
                let display_timestamp = timestamp_usec_to_display(&renderer.timestamp_usec);

                // メッセージ内容とrunsを構築（絵文字対応）
                let mut runs = Vec::new();
                let mut content_parts = Vec::new();

                if let Some(msg) = &renderer.message {
                    for run in &msg.runs {
                        if let Some(text) = run.get_text() {
                            runs.push(MessageRun::Text {
                                content: text.to_string(),
                            });
                            content_parts.push(text.to_string());
                        } else if let Some(emoji) = run.get_emoji() {
                            let image_url = emoji
                                .image
                                .thumbnails
                                .first()
                                .map(|t| t.url.clone())
                                .unwrap_or_default();
                            let alt_text =
                                if let Some(accessibility) = &emoji.image.accessibility {
                                    accessibility.accessibility_data.label.clone()
                                } else {
                                    format!(":{}: ", emoji.emoji_id)
                                };

                            runs.push(MessageRun::Emoji {
                                emoji_id: emoji.emoji_id.clone(),
                                image_url,
                                alt_text: alt_text.clone(),
                            });
                            content_parts.push(alt_text);
                        }
                    }
                }

                let content = content_parts.join("");

                // YouTubeから提供された色情報を抽出
                let superchat_colors = extract_superchat_colors(
                    renderer.header_background_color,
                    renderer.header_text_color,
                    renderer.body_background_color,
                    renderer.body_text_color,
                );

                Self {
                    id: renderer.id.clone(),
                    timestamp: display_timestamp,
                    timestamp_usec: renderer.timestamp_usec.clone(),
                    message_type: MessageType::SuperChat {
                        amount: renderer.purchase_amount_text.simple_text.clone(),
                    },
                    author: renderer.author_name.simple_text.clone(),
                    author_icon_url,
                    channel_id: renderer.author_external_channel_id.clone(),
                    content,
                    runs,
                    metadata: Some(MessageMetadata {
                        amount: Some(renderer.purchase_amount_text.simple_text.clone()),
                        badges,
                        badge_info,
                        color: None,
                        is_moderator,
                        is_verified,
                        superchat_colors: Some(superchat_colors),
                    }),
                    is_member,
                    comment_count: None, // StateManagerで後から設定される
                }
            }
            crate::get_live_chat::ChatItem::PaidSticker { renderer } => {
                let (badges, badge_info, is_member, is_moderator, is_verified) =
                    extract_badge_info(&renderer.author_badges);

                // アイコンURL抽出
                let author_icon_url = renderer
                    .author_photo
                    .thumbnails
                    .first()
                    .map(|thumbnail| thumbnail.url.clone());

                // タイムスタンプ変換（マイクロ秒 → 表示用）
                let display_timestamp = timestamp_usec_to_display(&renderer.timestamp_usec);

                // YouTubeから提供された色情報を抽出
                let superchat_colors = extract_supersticker_colors(
                    renderer.money_chip_background_color,
                    renderer.money_chip_text_color,
                );

                Self {
                    id: renderer.id.clone(),
                    timestamp: display_timestamp,
                    timestamp_usec: renderer.timestamp_usec.clone(),
                    message_type: MessageType::SuperSticker {
                        amount: renderer.purchase_amount_text.simple_text.clone(),
                    },
                    author: renderer.author_name.simple_text.clone(),
                    author_icon_url,
                    channel_id: renderer.author_external_channel_id.clone(),
                    content: format!(
                        "Super Sticker ({})",
                        renderer.purchase_amount_text.simple_text
                    ),
                    runs: Vec::new(), // SuperStickerは固定テキスト
                    metadata: Some(MessageMetadata {
                        amount: Some(renderer.purchase_amount_text.simple_text.clone()),
                        badges,
                        badge_info,
                        color: None,
                        is_moderator,
                        is_verified,
                        superchat_colors: Some(superchat_colors),
                    }),
                    is_member,
                    comment_count: None, // StateManagerで後から設定される
                }
            }
            crate::get_live_chat::ChatItem::MembershipItem { renderer } => {
                let (badges, badge_info, _is_member, is_moderator, is_verified) =
                    extract_badge_info(&renderer.author_badges);

                // アイコンURL抽出
                let author_icon_url = renderer
                    .author_photo
                    .thumbnails
                    .first()
                    .map(|thumbnail| thumbnail.url.clone());

                // タイムスタンプ変換（マイクロ秒 → 表示用）
                let display_timestamp = timestamp_usec_to_display(&renderer.timestamp_usec);

                // header_primary_text からメンバーシップ情報を抽出
                let header_primary = renderer
                    .header_primary_text
                    .as_ref()
                    .map(|msg| extract_message_text(&msg.runs))
                    .unwrap_or_default();

                // header_subtext からサブテキストを抽出
                let header_sub = renderer
                    .header_subtext
                    .as_ref()
                    .map(|msg| extract_message_text(&msg.runs))
                    .unwrap_or_default();

                // message からユーザーメッセージを抽出
                let user_message = renderer
                    .message
                    .as_ref()
                    .map(|msg| extract_message_text(&msg.runs))
                    .unwrap_or_default();

                // マイルストーン月数を抽出（「メンバー歴 X か月」などのパターン）
                let milestone_months = extract_milestone_months(&header_primary, &header_sub);

                // デバッグログ: マイルストーンチャット検証用
                debug!(
                    author = %renderer.author_name.simple_text,
                    header_primary = %header_primary,
                    header_sub = %header_sub,
                    user_message = %user_message,
                    milestone_months = ?milestone_months,
                    "Membership message received"
                );

                // コンテンツを生成
                let content = build_membership_content(
                    &header_primary,
                    &header_sub,
                    &user_message,
                    milestone_months,
                );

                // runs を構築（ユーザーメッセージがある場合）
                let runs = if let Some(msg) = &renderer.message {
                    msg.runs
                        .iter()
                        .filter_map(|run| {
                            if let Some(text) = run.get_text() {
                                Some(MessageRun::Text {
                                    content: text.to_string(),
                                })
                            } else if let Some(emoji) = run.get_emoji() {
                                let image_url = emoji
                                    .image
                                    .thumbnails
                                    .first()
                                    .map(|t| t.url.clone())
                                    .unwrap_or_default();
                                let alt_text =
                                    if let Some(accessibility) = &emoji.image.accessibility {
                                        accessibility.accessibility_data.label.clone()
                                    } else {
                                        format!("Emoji: {}", emoji.emoji_id)
                                    };
                                Some(MessageRun::Emoji {
                                    emoji_id: emoji.emoji_id.clone(),
                                    image_url,
                                    alt_text,
                                })
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                Self {
                    id: renderer.id.clone(),
                    timestamp: display_timestamp,
                    timestamp_usec: renderer.timestamp_usec.clone(),
                    message_type: MessageType::Membership { milestone_months },
                    author: renderer.author_name.simple_text.clone(),
                    author_icon_url,
                    channel_id: renderer.author_external_channel_id.clone(),
                    content,
                    runs,
                    metadata: Some(MessageMetadata {
                        amount: None,
                        badges,
                        badge_info,
                        color: None,
                        is_moderator,
                        is_verified,
                        superchat_colors: None,
                    }),
                    is_member: true,     // メンバーシップアイテムは常にメンバー
                    comment_count: None, // StateManagerで後から設定される
                }
            }
            crate::get_live_chat::ChatItem::SponsorshipsGiftPurchaseAnnouncement { renderer } => {
                let (badges, badge_info, is_member, is_moderator, is_verified) =
                    extract_badge_info(&renderer.author_badges);

                // アイコンURL抽出
                let author_icon_url = renderer
                    .author_photo
                    .thumbnails
                    .first()
                    .map(|thumbnail| thumbnail.url.clone());

                // タイムスタンプ変換
                let display_timestamp = timestamp_usec_to_display(&renderer.timestamp_usec);

                // header からギフト情報を抽出
                let header_text = extract_message_text(&renderer.header.runs);

                // ギフト数を抽出（「X 人にメンバーシップをギフト購入しました」などのパターン）
                let gift_count = extract_gift_count(&header_text);

                debug!(
                    author = %renderer.author_name.simple_text,
                    header_text = %header_text,
                    gift_count = gift_count,
                    "MembershipGift message received"
                );

                Self {
                    id: renderer.id.clone(),
                    timestamp: display_timestamp,
                    timestamp_usec: renderer.timestamp_usec.clone(),
                    message_type: MessageType::MembershipGift { gift_count },
                    author: renderer.author_name.simple_text.clone(),
                    author_icon_url,
                    channel_id: renderer.author_external_channel_id.clone(),
                    content: header_text,
                    runs: Vec::new(),
                    metadata: Some(MessageMetadata {
                        amount: None,
                        badges,
                        badge_info,
                        color: None,
                        is_moderator,
                        is_verified,
                        superchat_colors: None,
                    }),
                    is_member,
                    comment_count: None,
                }
            }
            crate::get_live_chat::ChatItem::SponsorshipsGiftRedemptionAnnouncement { renderer } => {
                // ギフトメンバーシップを受け取った人（新規メンバーとして扱う）
                let (badges, badge_info, _is_member, is_moderator, is_verified) =
                    extract_badge_info(&renderer.author_badges);

                let author_icon_url = renderer
                    .author_photo
                    .thumbnails
                    .first()
                    .map(|thumbnail| thumbnail.url.clone());

                let display_timestamp = timestamp_usec_to_display(&renderer.timestamp_usec);

                let message_text = extract_message_text(&renderer.message.runs);

                Self {
                    id: renderer.id.clone(),
                    timestamp: display_timestamp,
                    timestamp_usec: renderer.timestamp_usec.clone(),
                    message_type: MessageType::Membership {
                        milestone_months: None,
                    },
                    author: renderer.author_name.simple_text.clone(),
                    author_icon_url,
                    channel_id: renderer.author_external_channel_id.clone(),
                    content: message_text,
                    runs: Vec::new(),
                    metadata: Some(MessageMetadata {
                        amount: None,
                        badges,
                        badge_info,
                        color: None,
                        is_moderator,
                        is_verified,
                        superchat_colors: None,
                    }),
                    is_member: true,
                    comment_count: None,
                }
            }
            _ => {
                // システムメッセージ用のタイムスタンプ（現在時刻をマイクロ秒で）
                let now_usec = chrono::Utc::now().timestamp_micros().to_string();
                let display_timestamp = chrono::Utc::now().format("%H:%M:%S").to_string();

                Self {
                    id: format!("system_{}", now_usec),
                    timestamp: display_timestamp,
                    timestamp_usec: now_usec,
                    message_type: MessageType::System,
                    author: "System".to_string(),
                    author_icon_url: None, // Systemメッセージにはアイコンなし
                    channel_id: "".to_string(),
                    content: "Unknown message type".to_string(),
                    runs: Vec::new(), // Systemメッセージは固定テキスト
                    metadata: None,
                    is_member: false,
                    comment_count: None, // Systemメッセージにはカウントなし
                }
            }
        }
    }
}

/// タイムスタンプをマイクロ秒から表示用文字列に変換
fn timestamp_usec_to_display(timestamp_usec: &str) -> String {
    if let Ok(usec) = timestamp_usec.parse::<i64>() {
        // マイクロ秒をchrono DateTimeに変換
        let secs = usec / 1_000_000;
        let nsecs = ((usec % 1_000_000) * 1000) as u32;
        if let Some(dt) = chrono::DateTime::from_timestamp(secs, nsecs) {
            // ローカルタイムに変換して表示
            let local: chrono::DateTime<chrono::Local> = dt.into();
            return local.format("%H:%M:%S").to_string();
        }
    }
    // パース失敗時は現在時刻を使用
    chrono::Local::now().format("%H:%M:%S").to_string()
}

/// ARGB u64値をhex文字列に変換
fn argb_to_hex(argb: u64) -> String {
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// SuperChat/SuperStickerの色情報を抽出（SuperChat用）
fn extract_superchat_colors(
    header_background_color: u64,
    header_text_color: u64,
    body_background_color: u64,
    body_text_color: u64,
) -> SuperChatColors {
    SuperChatColors {
        header_background: argb_to_hex(header_background_color),
        header_text: argb_to_hex(header_text_color),
        body_background: argb_to_hex(body_background_color),
        body_text: argb_to_hex(body_text_color),
    }
}

/// SuperSticker用の色情報を抽出
fn extract_supersticker_colors(
    money_chip_background_color: u64,
    money_chip_text_color: u64,
) -> SuperChatColors {
    // SuperStickerはmoneyChipの色のみなので、header/bodyを同じ色で設定
    let bg = argb_to_hex(money_chip_background_color);
    let text = argb_to_hex(money_chip_text_color);
    SuperChatColors {
        header_background: bg.clone(),
        header_text: text.clone(),
        body_background: bg,
        body_text: text,
    }
}

/// バッジ情報からメンバーシップ・モデレーター・認証情報を抽出
fn extract_badge_info(
    author_badges: &[crate::get_live_chat::AuthorBadge],
) -> (Vec<String>, Vec<BadgeInfo>, bool, bool, bool) {
    let mut badges = Vec::new();
    let mut badge_info = Vec::new();
    let mut is_member = false;
    let mut is_moderator = false;
    let mut is_verified = false;

    for badge in author_badges {
        let tooltip = &badge.renderer.tooltip;
        let accessibility_label = &badge.renderer.accessibility.accessibility_data.label;

        badges.push(tooltip.clone());

        // バッジ画像URLを抽出
        let image_url = badge
            .renderer
            .custom_thumbnail
            .as_ref()
            .and_then(|image| image.thumbnails.first())
            .map(|thumbnail| thumbnail.url.clone());

        badge_info.push(BadgeInfo {
            tooltip: tooltip.clone(),
            image_url,
        });

        // メンバーシップバッジの判定（複数パターン）
        if tooltip.contains("メンバー")
            || tooltip.contains("Member")
            || accessibility_label.contains("メンバー")
            || accessibility_label.contains("Member")
            || tooltip.contains("新規メンバー")
            || tooltip.contains("New member")
        {
            is_member = true;
        }

        // モデレーターバッジの判定
        if tooltip.contains("モデレーター")
            || tooltip.contains("Moderator")
            || accessibility_label.contains("モデレーター")
            || accessibility_label.contains("Moderator")
        {
            is_moderator = true;
        }

        // 認証済みバッジの判定
        if tooltip.contains("認証")
            || tooltip.contains("Verified")
            || accessibility_label.contains("認証")
            || accessibility_label.contains("Verified")
        {
            is_verified = true;
        }
    }

    (badges, badge_info, is_member, is_moderator, is_verified)
}

/// Message の runs からテキストを連結して抽出
fn extract_message_text(runs: &[crate::get_live_chat::MessageRun]) -> String {
    runs.iter()
        .filter_map(|run| run.get_text().map(|s| s.to_string()))
        .collect::<Vec<_>>()
        .join("")
}

/// マイルストーン月数を抽出
/// 日本語: 「メンバー歴 X か月」「X か月のメンバー」など
/// 英語: "Member for X months", "X month membership milestone" など
fn extract_milestone_months(header_primary: &str, header_sub: &str) -> Option<u32> {
    // 日本語パターン: 数字 + 「か月」「ヶ月」「カ月」
    let japanese_patterns = [
        r"(\d+)\s*か月",
        r"(\d+)\s*ヶ月",
        r"(\d+)\s*カ月",
        r"メンバー歴\s*(\d+)",
    ];

    // 英語パターン
    let english_patterns = [
        r"(\d+)\s*month",
        r"(\d+)\s*year",
        r"member\s+for\s+(\d+)",
    ];

    let combined_text = format!("{} {}", header_primary, header_sub);
    let lower_text = combined_text.to_lowercase();

    // 日本語パターンをチェック
    for pattern in &japanese_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(caps) = re.captures(&combined_text) {
                if let Some(num_str) = caps.get(1) {
                    if let Ok(months) = num_str.as_str().parse::<u32>() {
                        if months > 0 {
                            return Some(months);
                        }
                    }
                }
            }
        }
    }

    // 英語パターンをチェック
    for pattern in &english_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(caps) = re.captures(&lower_text) {
                if let Some(num_str) = caps.get(1) {
                    if let Ok(num) = num_str.as_str().parse::<u32>() {
                        // year パターンの場合は12倍
                        if pattern.contains("year") && num > 0 {
                            return Some(num * 12);
                        } else if num > 0 {
                            return Some(num);
                        }
                    }
                }
            }
        }
    }

    None
}

/// ギフト数を抽出
/// 日本語: 「X 人にメンバーシップをギフト購入しました」など
/// 英語: "Gifted X memberships" など
fn extract_gift_count(header_text: &str) -> u32 {
    // 日本語パターン: 数字 + 「人」
    let japanese_patterns = [r"(\d+)\s*人に"];

    // 英語パターン
    let english_patterns = [r"[Gg]ifted\s+(\d+)", r"(\d+)\s+membership"];

    // 日本語パターンをチェック
    for pattern in &japanese_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(caps) = re.captures(header_text) {
                if let Some(num_str) = caps.get(1) {
                    if let Ok(count) = num_str.as_str().parse::<u32>() {
                        if count > 0 {
                            return count;
                        }
                    }
                }
            }
        }
    }

    // 英語パターンをチェック
    let lower_text = header_text.to_lowercase();
    for pattern in &english_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(caps) = re.captures(&lower_text) {
                if let Some(num_str) = caps.get(1) {
                    if let Ok(count) = num_str.as_str().parse::<u32>() {
                        if count > 0 {
                            return count;
                        }
                    }
                }
            }
        }
    }

    // パターンが見つからない場合はデフォルト1
    1
}

/// メンバーシップコンテンツを構築
fn build_membership_content(
    header_primary: &str,
    header_sub: &str,
    user_message: &str,
    milestone_months: Option<u32>,
) -> String {
    let mut parts = Vec::new();

    // ヘッダープライマリテキスト（「メンバー歴 X か月」など）
    if !header_primary.is_empty() {
        parts.push(header_primary.to_string());
    }

    // ヘッダーサブテキスト
    if !header_sub.is_empty() {
        parts.push(header_sub.to_string());
    }

    // ユーザーメッセージ
    if !user_message.is_empty() {
        if !parts.is_empty() {
            parts.push(format!(": {}", user_message));
        } else {
            parts.push(user_message.to_string());
        }
    }

    // コンテンツが空の場合はデフォルトメッセージ
    if parts.is_empty() {
        if milestone_months.is_some() {
            "Membership milestone!".to_string()
        } else {
            "New member!".to_string()
        }
    } else {
        parts.join(" ")
    }
}

/// アプリケーション状態
#[derive(Debug, Clone)]
pub struct AppState {
    pub url: String,
    pub output_file: String,
    pub auto_save_enabled: bool, // 自動保存のオン・オフ
    pub is_connected: bool,
    pub message_count: usize,
    pub request_count: usize,
    pub messages: Vec<GuiChatMessage>,
    pub active_tab: ActiveTab,

    /// チャット表示設定
    pub chat_display_config: crate::gui::unified_config::ChatDisplayConfig,

    /// ウィンドウ設定
    pub window: crate::gui::config_manager::WindowConfig,

    // 新しい保存設定
    pub save_raw_responses: bool,
    pub raw_response_file: String,
    pub max_raw_file_size_mb: u64,
    pub enable_file_rotation: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            url: String::new(),
            output_file: "live_chat.ndjson".to_string(),
            auto_save_enabled: false,
            is_connected: false,
            message_count: 0,
            request_count: 0,
            messages: Vec::new(),
            active_tab: ActiveTab::default(),
            chat_display_config: crate::gui::unified_config::ChatDisplayConfig::default(),
            window: crate::gui::config_manager::WindowConfig::default(),
            save_raw_responses: false,
            raw_response_file: "raw_responses.ndjson".to_string(),
            max_raw_file_size_mb: 100,
            enable_file_rotation: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveTab {
    Chat,
    Export,
    Raw,
    Revenue,
    SignalAnalysis, // Phase 4.1: Signal分析タブ

    // Phase 4.3: 追加バリアント（互換性のため）
    ChatMonitor,
    RevenueAnalytics,
    DataExport,
    Settings,
}

impl Default for ActiveTab {
    fn default() -> Self {
        Self::Chat
    }
}

impl ActiveTab {
    pub fn to_string(&self) -> &'static str {
        match self {
            ActiveTab::Chat => "Chat",
            ActiveTab::Export => "Export",
            ActiveTab::Raw => "Raw",
            ActiveTab::Revenue => "Revenue",
            ActiveTab::SignalAnalysis => "Signal Analysis",

            // Phase 4.3: 追加バリアント（互換性マッピング）
            ActiveTab::ChatMonitor => "Chat",
            ActiveTab::RevenueAnalytics => "Revenue",
            ActiveTab::DataExport => "Export",
            ActiveTab::Settings => "Settings",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ActiveTab::Chat => "💬",
            ActiveTab::Export => "📥",
            ActiveTab::Raw => "📄",
            ActiveTab::Revenue => "💰",
            ActiveTab::SignalAnalysis => "📊",

            // Phase 4.3: 追加バリアント（互換性マッピング）
            ActiveTab::ChatMonitor => "💬",
            ActiveTab::RevenueAnalytics => "💰",
            ActiveTab::DataExport => "📥",
            ActiveTab::Settings => "⚙️",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ActiveTab::Chat => "Monitor real-time YouTube live chat messages",
            ActiveTab::Export => "Export and save chat data in various formats",
            ActiveTab::Raw => "Save raw responses from YouTube",
            ActiveTab::Revenue => "Track SuperChat revenue and membership earnings",
            ActiveTab::SignalAnalysis => "Analyze chat data for patterns and insights",

            // Phase 4.3: 追加バリアント（互換性マッピング）
            ActiveTab::ChatMonitor => "Monitor real-time YouTube live chat messages",
            ActiveTab::RevenueAnalytics => "Track SuperChat revenue and membership earnings",
            ActiveTab::DataExport => "Export and save chat data in various formats",
            ActiveTab::Settings => "Application settings and configuration",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_milestone_months_japanese_patterns() {
        // 「X か月」パターン
        assert_eq!(
            extract_milestone_months("メンバー歴 12 か月", ""),
            Some(12)
        );
        assert_eq!(extract_milestone_months("メンバー歴 1 か月", ""), Some(1));
        assert_eq!(extract_milestone_months("6か月", ""), Some(6));

        // 「X ヶ月」パターン
        assert_eq!(extract_milestone_months("メンバー歴 24 ヶ月", ""), Some(24));
        assert_eq!(extract_milestone_months("12ヶ月のメンバー", ""), Some(12));

        // 「X カ月」パターン
        assert_eq!(extract_milestone_months("メンバー歴 3 カ月", ""), Some(3));

        // ヘッダーサブテキストに含まれるパターン
        assert_eq!(
            extract_milestone_months("", "メンバー歴 6 か月"),
            Some(6)
        );

        // 両方に情報がある場合（header_primaryを優先）
        assert_eq!(
            extract_milestone_months("メンバー歴 12 か月", "メンバー歴 6 か月"),
            Some(12)
        );
    }

    #[test]
    fn test_extract_milestone_months_english_patterns() {
        // "X months" パターン
        assert_eq!(
            extract_milestone_months("Member for 12 months", ""),
            Some(12)
        );
        assert_eq!(extract_milestone_months("6 month milestone", ""), Some(6));
        assert_eq!(extract_milestone_months("1 month", ""), Some(1));

        // "X year(s)" パターン（12倍される）
        assert_eq!(extract_milestone_months("Member for 1 year", ""), Some(12));
        assert_eq!(
            extract_milestone_months("2 year membership milestone", ""),
            Some(24)
        );

        // 大文字小文字を区別しない
        assert_eq!(
            extract_milestone_months("MEMBER FOR 3 MONTHS", ""),
            Some(3)
        );
    }

    #[test]
    fn test_extract_milestone_months_no_match() {
        // マイルストーンではないパターン（新規メンバー）
        assert_eq!(
            extract_milestone_months("", "Welcome to the channel!"),
            None
        );
        assert_eq!(
            extract_milestone_months("新規メンバー", "チャンネルへようこそ"),
            None
        );

        // 空の入力
        assert_eq!(extract_milestone_months("", ""), None);
    }

    #[test]
    fn test_build_membership_content_new_member() {
        // 新規メンバー（マイルストーンなし）
        let content = build_membership_content("", "", "", None);
        assert_eq!(content, "New member!");

        // サブテキストのみ
        let content =
            build_membership_content("", "Welcome to the channel!", "", None);
        assert_eq!(content, "Welcome to the channel!");
    }

    #[test]
    fn test_build_membership_content_milestone() {
        // マイルストーン（ヘッダープライマリ + サブテキスト）
        let content = build_membership_content(
            "メンバー歴 12 か月",
            "おめでとうございます",
            "",
            Some(12),
        );
        assert_eq!(content, "メンバー歴 12 か月 おめでとうございます");

        // マイルストーン + ユーザーメッセージ
        let content = build_membership_content(
            "メンバー歴 6 か月",
            "",
            "いつもありがとう！",
            Some(6),
        );
        assert_eq!(content, "メンバー歴 6 か月 : いつもありがとう！");

        // マイルストーンでテキストが空の場合
        let content = build_membership_content("", "", "", Some(12));
        assert_eq!(content, "Membership milestone!");
    }

    #[test]
    fn test_message_type_as_string() {
        // 新規メンバー
        let msg_type = MessageType::Membership {
            milestone_months: None,
        };
        assert_eq!(msg_type.as_string(), "membership");

        // マイルストーン
        let msg_type = MessageType::Membership {
            milestone_months: Some(12),
        };
        assert_eq!(msg_type.as_string(), "membership-milestone");

        // その他のタイプ
        assert_eq!(MessageType::Text.as_string(), "text");
        assert_eq!(
            MessageType::SuperChat {
                amount: "¥500".to_string()
            }
            .as_string(),
            "super-chat"
        );
    }

    /// 実際のYouTubeレスポンスデータを使ったスーパーチャット変換テスト
    /// このテストは2024年12月の実際の配信から取得したデータを使用
    #[test]
    fn test_superchat_with_emoji_only_message_from_real_data() {
        // 実際のスーパーチャットデータ（絵文字のみのメッセージ）
        let json = r#"{
            "id": "ChwKGkNQS2Ywb3FjNVpFREZUckN3Z1FkNC00QUFB",
            "message": {
                "runs": [
                    {
                        "text": null,
                        "emoji": {
                            "emojiId": "🍼",
                            "image": {
                                "thumbnails": [
                                    {
                                        "url": "https://fonts.gstatic.com/s/e/notoemoji/15.1/1f37c/72.png",
                                        "width": null,
                                        "height": null
                                    }
                                ],
                                "accessibility": {
                                    "accessibilityData": {
                                        "label": "🍼"
                                    }
                                }
                            },
                            "searchTerms": ["baby", "bottle"],
                            "shortcuts": [":baby_bottle:"],
                            "isCustomEmoji": false
                        }
                    }
                ]
            },
            "authorName": { "simpleText": "@なんた-r5v" },
            "authorPhoto": {
                "thumbnails": [
                    { "url": "https://example.com/photo.jpg", "width": 32, "height": 32 }
                ]
            },
            "timestampUsec": "1767094535233715",
            "authorExternalChannelId": "UCS4XO7apDrR8MDp2KYHfKLw",
            "purchaseAmountText": { "simpleText": "¥200" },
            "authorBadges": [
                {
                    "liveChatAuthorBadgeRenderer": {
                        "accessibility": {
                            "accessibilityData": { "label": "Member (1 year)" }
                        },
                        "tooltip": "Member (1 year)",
                        "customThumbnail": {
                            "thumbnails": [
                                { "url": "https://example.com/badge.png", "width": 16, "height": 16 }
                            ],
                            "accessibility": null
                        }
                    }
                }
            ],
            "trackingParams": "test",
            "headerBackgroundColor": 4278237396,
            "headerTextColor": 4278190080,
            "bodyBackgroundColor": 4278248959,
            "bodyTextColor": 4278190080
        }"#;

        let renderer: crate::get_live_chat::LiveChatPaidMessageRenderer =
            serde_json::from_str(json).expect("Failed to parse SuperChat JSON");

        let chat_item = crate::get_live_chat::ChatItem::PaidMessage { renderer };
        let gui_message = GuiChatMessage::from(chat_item);

        // 検証: メッセージタイプがSuperChatで金額が正しい
        assert!(matches!(
            gui_message.message_type,
            MessageType::SuperChat { ref amount } if amount == "¥200"
        ));

        // 検証: 著者名が正しい
        assert_eq!(gui_message.author, "@なんた-r5v");

        // 検証: runsに絵文字が含まれている（修正前は空だった）
        assert_eq!(gui_message.runs.len(), 1);
        assert!(matches!(
            &gui_message.runs[0],
            MessageRun::Emoji { emoji_id, alt_text, .. }
            if emoji_id == "🍼" && alt_text == "🍼"
        ));

        // 検証: contentに絵文字のalt_textが含まれている
        assert!(gui_message.content.contains("🍼"));

        // 検証: メンバーとして認識されている
        assert!(gui_message.is_member);
    }

    /// 実際のYouTubeレスポンスデータを使ったスーパーステッカー変換テスト
    #[test]
    fn test_supersticker_from_real_data() {
        // 実際のスーパーステッカーデータ
        let json = r#"{
            "id": "ChwKGkNOU0oySS1iNVpFREZmUEN3Z1FkeHhVZGZB",
            "authorName": { "simpleText": "@しょうや-x5y" },
            "authorPhoto": {
                "thumbnails": [
                    { "url": "https://example.com/photo.jpg", "width": 32, "height": 32 }
                ]
            },
            "timestampUsec": "1767094289588094",
            "authorExternalChannelId": "UCj8UiIHFrFLwFGcYKeB3Rtg",
            "purchaseAmountText": { "simpleText": "¥140" },
            "sticker": {
                "thumbnails": [
                    { "url": "https://example.com/sticker.png", "width": 40, "height": 40 }
                ]
            },
            "authorBadges": [
                {
                    "liveChatAuthorBadgeRenderer": {
                        "accessibility": {
                            "accessibilityData": { "label": "Member (6 months)" }
                        },
                        "tooltip": "Member (6 months)",
                        "customThumbnail": {
                            "thumbnails": [
                                { "url": "https://example.com/badge.png", "width": 16, "height": 16 }
                            ],
                            "accessibility": null
                        }
                    }
                }
            ],
            "trackingParams": "test",
            "moneyChipBackgroundColor": 4280191205,
            "moneyChipTextColor": 4294967295
        }"#;

        let renderer: crate::get_live_chat::LiveChatPaidStickerRenderer =
            serde_json::from_str(json).expect("Failed to parse SuperSticker JSON");

        let chat_item = crate::get_live_chat::ChatItem::PaidSticker { renderer };
        let gui_message = GuiChatMessage::from(chat_item);

        // 検証: メッセージタイプがSuperStickerで金額が正しい
        assert!(matches!(
            gui_message.message_type,
            MessageType::SuperSticker { ref amount } if amount == "¥140"
        ));

        // 検証: 著者名が正しい
        assert_eq!(gui_message.author, "@しょうや-x5y");

        // 検証: contentにSuperStickerと金額が含まれている
        assert!(gui_message.content.contains("Super Sticker"));
        assert!(gui_message.content.contains("¥140"));

        // 検証: メンバーとして認識されている
        assert!(gui_message.is_member);
    }
}
