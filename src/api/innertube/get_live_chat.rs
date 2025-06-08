//! YouTube Live Chat API client implementation.
//!
//! This module provides data structures and functions for parsing and processing
//! YouTube live chat messages from the InnerTube API.

use serde::{Deserialize, Serialize};

/// Entry in a response stream containing a timestamp and the live chat response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseEntry {
    /// Unix timestamp when the entry was received
    pub timestamp: u64,
    /// The actual response from the YouTube API
    pub response: GetLiveChatResponse,
}

/// Response from the YouTube Live Chat API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLiveChatResponse {
    /// The contents of the continuation, including chat messages
    #[serde(rename = "continuationContents")]
    pub continuation_contents: ContinuationContents,
}

/// Container for the live chat continuation data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuationContents {
    /// The live chat continuation containing actions and the next continuation token
    #[serde(rename = "liveChatContinuation")]
    pub live_chat_continuation: LiveChatContinuation,
}

/// Live chat continuation containing actions and tokens for the next request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatContinuation {
    /// Token for fetching the next batch of messages (may be absent in some responses)
    pub continuation: Option<Continuation>,
    /// Array of actions like new messages, deletions, etc.
    #[serde(default)]
    pub actions: Vec<Action>,
    /// Array of continuation data for future requests
    #[serde(default)]
    pub continuations: Vec<serde_json::Value>,
}

/// Continuation token for fetching the next batch of messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Continuation(pub String);

/// A message containing a sequence of text and/or emoji runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Sequence of text fragments and emojis that make up the message
    pub runs: Vec<MessageRun>,
}

/// A fragment of a message, containing either text or an emoji.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRun {
    /// Text content if this is a text run
    pub text: Option<String>,
    /// Emoji content if this is an emoji run
    pub emoji: Option<Emoji>,
}

impl MessageRun {
    /// Get the text content if present
    pub fn get_text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Get the emoji content if present
    pub fn get_emoji(&self) -> Option<&Emoji> {
        self.emoji.as_ref()
    }
}

/// Emoji data structure for custom and standard emojis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emoji {
    /// Identifier for the emoji
    #[serde(rename = "emojiId", default)]
    pub emoji_id: String,
    /// Image data for the emoji
    pub image: Image,
    /// Search terms associated with this emoji
    #[serde(rename = "searchTerms", default)]
    pub search_terms: Vec<String>,
    /// Shortcut text strings to input this emoji
    #[serde(default)]
    pub shortcuts: Vec<String>,
    /// Whether this is a custom emoji (vs a standard Unicode emoji)
    #[serde(rename = "isCustomEmoji", default)]
    pub is_custom_emoji: bool,
}

/// Image data with thumbnails and accessibility information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    /// Different sizes of the image
    pub thumbnails: Vec<Thumbnail>,
    /// Accessibility information for screen readers
    pub accessibility: Option<Accessibility>,
}

/// Thumbnail information for an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    /// URL of the thumbnail image
    pub url: String,
    /// Width of the thumbnail in pixels
    pub width: Option<u32>,
    /// Height of the thumbnail in pixels
    pub height: Option<u32>,
}

/// Accessibility information for screen readers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Accessibility {
    /// Accessibility data including labels
    #[serde(rename = "accessibilityData")]
    pub accessibility_data: AccessibilityData,
}

/// Data for accessibility features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityData {
    /// Text label for screen readers
    pub label: String,
}

/// Author name information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorName {
    /// The display name of the author
    #[serde(rename = "simpleText")]
    pub simple_text: String,
}

/// Author profile photo information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorPhoto {
    /// Different sizes of the author's profile photo
    pub thumbnails: Vec<Thumbnail>,
}

/// Context menu accessibility information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuAccessibility {
    /// Accessibility data for the context menu
    #[serde(rename = "accessibilityData")]
    pub accessibility_data: AccessibilityData,
}

/// Web command metadata for context menu actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebCommandMetadata {
    /// Whether to ignore navigation events
    #[serde(rename = "ignoreNavigation")]
    pub ignore_navigation: bool,
}

/// Command metadata for context menu actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandMetadata {
    /// Web-specific command metadata
    #[serde(rename = "webCommandMetadata")]
    pub web_command_metadata: WebCommandMetadata,
}

/// Endpoint for context menu actions on live chat items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatItemContextMenuEndpoint {
    /// Parameters for the endpoint
    pub params: String,
}

/// Endpoint for context menu interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuEndpoint {
    /// Tracking parameters for the click
    #[serde(
        rename = "clickTrackingParams",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub click_tracking_params: Option<String>,
    /// Metadata for the command
    #[serde(rename = "commandMetadata")]
    pub command_metadata: CommandMetadata,
    /// Live chat item context menu endpoint
    #[serde(rename = "liveChatItemContextMenuEndpoint")]
    pub live_chat_item_context_menu_endpoint: LiveChatItemContextMenuEndpoint,
}

/// Renderer for a standard text message in live chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatTextMessageRenderer {
    /// Unique identifier for the message
    pub id: String,
    /// Content of the message with text and/or emojis
    pub message: Message,
    /// Name of the message author
    #[serde(rename = "authorName")]
    pub author_name: AuthorName,
    /// Profile photo of the message author
    #[serde(rename = "authorPhoto")]
    pub author_photo: AuthorPhoto,
    /// Timestamp in microseconds when the message was sent
    #[serde(rename = "timestampUsec")]
    pub timestamp_usec: String,
    /// External channel ID of the author
    #[serde(rename = "authorExternalChannelId")]
    pub author_external_channel_id: String,
    /// Badges displayed next to the author's name
    #[serde(rename = "authorBadges", default)]
    pub author_badges: Vec<AuthorBadge>,
    /// Tracking parameters for analytics
    #[serde(
        rename = "trackingParams",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub tracking_params: Option<String>,
    /// Endpoint for context menu actions
    #[serde(
        rename = "contextMenuEndpoint",
        skip_serializing_if = "Option::is_none"
    )]
    pub context_menu_endpoint: Option<ContextMenuEndpoint>,
    /// Accessibility information for the context menu
    #[serde(
        rename = "contextMenuAccessibility",
        skip_serializing_if = "Option::is_none"
    )]
    pub context_menu_accessibility: Option<ContextMenuAccessibility>,
}

/// Badge displayed next to an author's name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorBadge {
    /// Renderer for the author badge
    #[serde(rename = "liveChatAuthorBadgeRenderer")]
    pub renderer: LiveChatAuthorBadgeRenderer,
}

/// Renderer for an author badge in live chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatAuthorBadgeRenderer {
    /// Accessibility information for the badge
    pub accessibility: Accessibility,
    /// Tooltip text displayed on hover
    pub tooltip: String,
    /// Custom thumbnail for the badge, if any
    #[serde(rename = "customThumbnail", skip_serializing_if = "Option::is_none")]
    pub custom_thumbnail: Option<Image>,
}

/// Renderer for a paid message (Super Chat) in live chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatPaidMessageRenderer {
    /// Unique identifier for the message
    pub id: String,
    /// Content of the message, if any
    pub message: Option<Message>,
    /// Name of the message author
    #[serde(rename = "authorName")]
    pub author_name: AuthorName,
    /// Profile photo of the message author
    #[serde(rename = "authorPhoto")]
    pub author_photo: AuthorPhoto,
    /// Timestamp in microseconds when the message was sent
    #[serde(rename = "timestampUsec")]
    pub timestamp_usec: String,
    /// External channel ID of the author
    #[serde(rename = "authorExternalChannelId")]
    pub author_external_channel_id: String,
    /// Text displaying the purchase amount
    #[serde(rename = "purchaseAmountText")]
    pub purchase_amount_text: SimpleText,
    /// Badges displayed next to the author's name
    #[serde(rename = "authorBadges", default)]
    pub author_badges: Vec<AuthorBadge>,
    /// Tracking parameters for analytics
    #[serde(rename = "trackingParams")]
    pub tracking_params: String,
    /// Background color for the header section
    #[serde(rename = "headerBackgroundColor")]
    pub header_background_color: u64,
    /// Text color for the header section
    #[serde(rename = "headerTextColor")]
    pub header_text_color: u64,
    /// Background color for the body section
    #[serde(rename = "bodyBackgroundColor")]
    pub body_background_color: u64,
    /// Text color for the body section
    #[serde(rename = "bodyTextColor")]
    pub body_text_color: u64,
}

/// Simple text container with plain text content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleText {
    /// The plain text content
    #[serde(rename = "simpleText")]
    pub simple_text: String,
}

/// Renderer for a paid sticker (Super Sticker) in live chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatPaidStickerRenderer {
    /// Unique identifier for the message
    pub id: String,
    /// Name of the message author
    #[serde(rename = "authorName")]
    pub author_name: AuthorName,
    /// Profile photo of the message author
    #[serde(rename = "authorPhoto")]
    pub author_photo: AuthorPhoto,
    /// Timestamp in microseconds when the message was sent
    #[serde(rename = "timestampUsec")]
    pub timestamp_usec: String,
    /// External channel ID of the author
    #[serde(rename = "authorExternalChannelId")]
    pub author_external_channel_id: String,
    /// Text displaying the purchase amount
    #[serde(rename = "purchaseAmountText")]
    pub purchase_amount_text: SimpleText,
    /// The sticker image information
    #[serde(rename = "sticker")]
    pub sticker: Sticker,
    /// Badges displayed next to the author's name
    #[serde(rename = "authorBadges", default)]
    pub author_badges: Vec<AuthorBadge>,
    /// Tracking parameters for analytics
    #[serde(rename = "trackingParams")]
    pub tracking_params: String,
    /// Background color for the money chip
    #[serde(rename = "moneyChipBackgroundColor")]
    pub money_chip_background_color: u64,
    /// Text color for the money chip
    #[serde(rename = "moneyChipTextColor")]
    pub money_chip_text_color: u64,
}

/// Sticker image information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sticker {
    /// Different sizes of the sticker image
    pub thumbnails: Vec<Thumbnail>,
}

/// Renderer for a membership item in live chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatMembershipItemRenderer {
    /// Unique identifier for the message
    pub id: String,
    /// Name of the message author
    #[serde(rename = "authorName")]
    pub author_name: AuthorName,
    /// Profile photo of the message author
    #[serde(rename = "authorPhoto")]
    pub author_photo: AuthorPhoto,
    /// Timestamp in microseconds when the message was sent
    #[serde(rename = "timestampUsec")]
    pub timestamp_usec: String,
    /// External channel ID of the author
    #[serde(rename = "authorExternalChannelId")]
    pub author_external_channel_id: String,
    /// Primary text in the header section
    #[serde(rename = "headerPrimaryText", skip_serializing_if = "Option::is_none")]
    pub header_primary_text: Option<Message>,
    /// Secondary text in the header section
    #[serde(rename = "headerSubtext", skip_serializing_if = "Option::is_none")]
    pub header_subtext: Option<Message>,
    /// Content of the message
    pub message: Option<Message>,
    /// Badges displayed next to the author's name
    #[serde(rename = "authorBadges", default)]
    pub author_badges: Vec<AuthorBadge>,
    /// Tracking parameters for analytics
    #[serde(rename = "trackingParams")]
    pub tracking_params: String,
}

/// Renderer for a viewer engagement message in live chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatViewerEngagementMessageRenderer {
    /// Unique identifier for the message
    pub id: String,
    /// Content of the message
    pub message: Message,
    /// Timestamp in microseconds when the message was sent
    #[serde(rename = "timestampUsec")]
    pub timestamp_usec: String,
    /// Tracking parameters for analytics
    #[serde(rename = "trackingParams")]
    pub tracking_params: String,
    /// Icon displayed with the message
    pub icon: Option<Icon>,
    /// Button for user interaction, if any
    #[serde(rename = "actionButton")]
    pub action_button: Option<ActionButton>,
}

/// Icon information for viewer engagement messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon {
    /// Type of icon to display
    #[serde(rename = "iconType")]
    pub icon_type: String,
}

/// Action button for viewer engagement messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionButton {
    /// Renderer for the button
    #[serde(rename = "buttonRenderer")]
    pub button_renderer: ButtonRenderer,
}

/// Renderer for a button in live chat UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonRenderer {
    /// Text displayed on the button
    pub text: ButtonText,
    /// Visual style of the button
    pub style: String,
    /// Size of the button
    pub size: String,
    /// Whether the button is disabled
    #[serde(rename = "isDisabled")]
    pub is_disabled: bool,
    /// Tracking parameters for analytics
    #[serde(rename = "trackingParams")]
    pub tracking_params: String,
}

/// Text displayed on a button.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonText {
    /// The plain text content
    #[serde(rename = "simpleText")]
    pub simple_text: String,
}

/// Renderer for a sponsorships gift purchase announcement in live chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatSponsorshipsGiftPurchaseAnnouncementRenderer {
    /// Unique identifier for the message
    pub id: String,
    /// Name of the message author
    #[serde(rename = "authorName")]
    pub author_name: AuthorName,
    /// Profile photo of the message author
    #[serde(rename = "authorPhoto")]
    pub author_photo: AuthorPhoto,
    /// Timestamp in microseconds when the message was sent
    #[serde(rename = "timestampUsec")]
    pub timestamp_usec: String,
    /// External channel ID of the author
    #[serde(rename = "authorExternalChannelId")]
    pub author_external_channel_id: String,
    /// Header content describing the gift purchase
    pub header: Message,
    /// Badges displayed next to the author's name
    #[serde(rename = "authorBadges", default)]
    pub author_badges: Vec<AuthorBadge>,
    /// Tracking parameters for analytics
    #[serde(rename = "trackingParams")]
    pub tracking_params: String,
}

/// Renderer for a sponsorships gift redemption announcement in live chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatSponsorshipsGiftRedemptionAnnouncementRenderer {
    /// Unique identifier for the message
    pub id: String,
    /// Name of the message author
    #[serde(rename = "authorName")]
    pub author_name: AuthorName,
    /// Profile photo of the message author
    #[serde(rename = "authorPhoto")]
    pub author_photo: AuthorPhoto,
    /// Timestamp in microseconds when the message was sent
    #[serde(rename = "timestampUsec")]
    pub timestamp_usec: String,
    /// External channel ID of the author
    #[serde(rename = "authorExternalChannelId")]
    pub author_external_channel_id: String,
    /// Message content describing the gift redemption
    pub message: Message,
    /// Badges displayed next to the author's name
    #[serde(rename = "authorBadges", default)]
    pub author_badges: Vec<AuthorBadge>,
    /// Tracking parameters for analytics
    #[serde(rename = "trackingParams")]
    pub tracking_params: String,
}

/// Wrapper for an AddChatItemAction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddChatItemActionWrapper {
    /// The action to add a chat item
    #[serde(rename = "addChatItemAction")]
    pub action: AddChatItemAction,
    /// Tracking parameters for analytics
    #[serde(rename = "clickTrackingParams")]
    pub click_tracking_params: Option<String>,
}

/// Action to add a live chat ticker item (pinned message).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddLiveChatTickerItemAction {
    /// The ticker item to add
    pub item: TickerItem,
    /// Duration in seconds that the ticker should be displayed
    #[serde(rename = "durationSec")]
    pub duration_sec: String,
}

/// Wrapper for an AddLiveChatTickerItemAction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddLiveChatTickerItemActionWrapper {
    /// The action to add a ticker item
    #[serde(rename = "addLiveChatTickerItemAction")]
    pub action: AddLiveChatTickerItemAction,
    /// Tracking parameters for analytics
    #[serde(rename = "clickTrackingParams")]
    pub click_tracking_params: Option<String>,
}

/// Enum representing different types of ticker items.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TickerItem {
    /// Paid message ticker (Super Chat)
    PaidMessage {
        /// Renderer for the paid message ticker
        #[serde(rename = "liveChatTickerPaidMessageItemRenderer")]
        renderer: LiveChatTickerPaidMessageItemRenderer,
    },
    /// Paid sticker ticker (Super Sticker)
    PaidSticker {
        /// Renderer for the paid sticker ticker
        #[serde(rename = "liveChatTickerPaidStickerItemRenderer")]
        renderer: LiveChatTickerPaidStickerItemRenderer,
    },
    /// Sponsor/membership ticker
    Sponsor {
        /// Renderer for the sponsor ticker
        #[serde(rename = "liveChatTickerSponsorItemRenderer")]
        renderer: LiveChatTickerSponsorItemRenderer,
    },
    /// Unknown ticker item type
    Unknown(serde_json::Value),
}

impl TickerItem {
    /// Returns a string identifying the type of ticker item
    pub fn get_type(&self) -> &'static str {
        match self {
            TickerItem::PaidMessage { .. } => "paidMessage",
            TickerItem::PaidSticker { .. } => "paidSticker",
            TickerItem::Sponsor { .. } => "sponsor",
            TickerItem::Unknown(_) => "unknown",
        }
    }
}

/// Action to remove a chat item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveChatItemAction {
    /// ID of the item to remove
    #[serde(rename = "targetItemId")]
    pub target_item_id: String,
}

/// Wrapper for a RemoveChatItemAction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveChatItemActionWrapper {
    /// The action to remove a chat item
    #[serde(rename = "removeChatItemAction")]
    pub action: RemoveChatItemAction,
    /// Tracking parameters for analytics
    #[serde(rename = "clickTrackingParams")]
    pub click_tracking_params: Option<String>,
}

/// Renderer for a ticker paid message item (pinned Super Chat).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatTickerPaidMessageItemRenderer {
    /// Unique identifier for the ticker item
    pub id: String,
    /// External channel ID of the author
    #[serde(rename = "authorExternalChannelId")]
    pub author_external_channel_id: String,
    /// Profile photo of the message author
    #[serde(rename = "authorPhoto")]
    pub author_photo: AuthorPhoto,
    /// Username of the author, if available
    #[serde(rename = "authorUsername", skip_serializing_if = "Option::is_none")]
    pub author_username: Option<SimpleText>,
    /// Starting background color for the ticker animation
    #[serde(rename = "startBackgroundColor")]
    pub start_background_color: u64,
    /// Ending background color for the ticker animation
    #[serde(rename = "endBackgroundColor")]
    pub end_background_color: u64,
    /// Duration in seconds that the ticker should be displayed
    #[serde(rename = "durationSec")]
    pub duration_sec: u64,
    /// Total duration in seconds for the ticker's lifecycle
    #[serde(rename = "fullDurationSec")]
    pub full_duration_sec: u64,
    /// Text color for the amount display
    #[serde(rename = "amountTextColor")]
    pub amount_text_color: u64,
    /// Tracking parameters for analytics
    #[serde(rename = "trackingParams")]
    pub tracking_params: String,
    /// Origin of the animation effect
    #[serde(rename = "animationOrigin", skip_serializing_if = "Option::is_none")]
    pub animation_origin: Option<String>,
    /// Dynamic state data for the ticker
    #[serde(rename = "dynamicStateData", skip_serializing_if = "Option::is_none")]
    pub dynamic_state_data: Option<serde_json::Value>,
    /// Endpoint for showing the item details
    #[serde(rename = "showItemEndpoint")]
    pub show_item_endpoint: ShowItemEndpoint,
    /// Command to open the engagement panel
    #[serde(
        rename = "openEngagementPanelCommand",
        skip_serializing_if = "Option::is_none"
    )]
    pub open_engagement_panel_command: Option<serde_json::Value>,
}

/// Renderer for a ticker paid sticker item (pinned Super Sticker).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatTickerPaidStickerItemRenderer {
    /// Unique identifier for the ticker item
    pub id: String,
    /// External channel ID of the author
    #[serde(rename = "authorExternalChannelId")]
    pub author_external_channel_id: String,
    /// Profile photo of the message author
    #[serde(rename = "authorPhoto")]
    pub author_photo: AuthorPhoto,
    /// Starting background color for the ticker animation
    #[serde(rename = "startBackgroundColor")]
    pub start_background_color: u64,
    /// Ending background color for the ticker animation
    #[serde(rename = "endBackgroundColor")]
    pub end_background_color: u64,
    /// Duration in seconds that the ticker should be displayed
    #[serde(rename = "durationSec")]
    pub duration_sec: u64,
    /// Total duration in seconds for the ticker's lifecycle
    #[serde(rename = "fullDurationSec")]
    pub full_duration_sec: u64,
    /// Tracking parameters for analytics
    #[serde(rename = "trackingParams")]
    pub tracking_params: String,
    /// Endpoint for showing the item details
    #[serde(rename = "showItemEndpoint")]
    pub show_item_endpoint: ShowItemEndpoint,
    /// Username of the author, if available
    #[serde(rename = "authorUsername", skip_serializing_if = "Option::is_none")]
    pub author_username: Option<SimpleText>,
    /// Origin of the animation effect
    #[serde(rename = "animationOrigin", skip_serializing_if = "Option::is_none")]
    pub animation_origin: Option<String>,
    /// Dynamic state data for the ticker
    #[serde(rename = "dynamicStateData", skip_serializing_if = "Option::is_none")]
    pub dynamic_state_data: Option<serde_json::Value>,
}

/// Renderer for a ticker sponsor item (pinned membership).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatTickerSponsorItemRenderer {
    /// Unique identifier for the ticker item
    pub id: String,
    /// External channel ID of the author
    #[serde(rename = "authorExternalChannelId")]
    pub author_external_channel_id: String,
    /// Profile photo of the sponsor
    #[serde(rename = "sponsorPhoto")]
    pub sponsor_photo: AuthorPhoto,
    /// Detail text describing the sponsorship
    #[serde(rename = "detailText")]
    pub detail_text: DetailText,
    /// Text color for the detail text
    #[serde(rename = "detailTextColor")]
    pub detail_text_color: u64,
    /// Starting background color for the ticker animation
    #[serde(rename = "startBackgroundColor")]
    pub start_background_color: u64,
    /// Ending background color for the ticker animation
    #[serde(rename = "endBackgroundColor")]
    pub end_background_color: u64,
    /// Duration in seconds that the ticker should be displayed
    #[serde(rename = "durationSec")]
    pub duration_sec: u64,
    /// Total duration in seconds for the ticker's lifecycle
    #[serde(rename = "fullDurationSec")]
    pub full_duration_sec: u64,
    /// Tracking parameters for analytics
    #[serde(rename = "trackingParams")]
    pub tracking_params: String,
    /// Endpoint for showing the item details
    #[serde(rename = "showItemEndpoint")]
    pub show_item_endpoint: ShowItemEndpoint,
    /// Dynamic state data for the ticker
    #[serde(rename = "dynamicStateData", skip_serializing_if = "Option::is_none")]
    pub dynamic_state_data: Option<serde_json::Value>,
}

/// Detail text for a ticker sponsor item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailText {
    /// Accessibility information
    pub accessibility: Option<Accessibility>,
    /// Simple text content
    #[serde(rename = "simpleText")]
    pub simple_text: Option<String>,
    /// Runs of text and emojis
    #[serde(default)]
    pub runs: Vec<MessageRun>,
}

/// Endpoint for showing a live chat item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowItemEndpoint {
    /// Endpoint for showing the live chat item
    #[serde(rename = "showLiveChatItemEndpoint")]
    pub show_live_chat_item_endpoint: ShowLiveChatItemEndpoint,
}

/// Endpoint details for showing a live chat item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowLiveChatItemEndpoint {
    /// Renderer for the live chat item
    #[serde(rename = "renderer")]
    pub renderer: serde_json::Value,
}

/// Enum representing different types of actions in the live chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Action {
    /// Action to add a new chat message
    AddChatItem(AddChatItemActionWrapper),
    /// Action to add a ticker item (pinned message)
    AddLiveChatTickerItem(AddLiveChatTickerItemActionWrapper),
    /// Action to remove a chat message
    RemoveChatItem(RemoveChatItemActionWrapper),
    /// Unknown action type
    Unknown(serde_json::Value),
}

/// Enum representing different types of chat items.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatItem {
    /// Standard text message in chat
    TextMessage {
        #[serde(rename = "liveChatTextMessageRenderer")]
        renderer: LiveChatTextMessageRenderer,
    },
    /// Paid message (Super Chat)
    PaidMessage {
        #[serde(rename = "liveChatPaidMessageRenderer")]
        renderer: LiveChatPaidMessageRenderer,
    },
    /// Paid sticker (Super Sticker)
    PaidSticker {
        #[serde(rename = "liveChatPaidStickerRenderer")]
        renderer: LiveChatPaidStickerRenderer,
    },
    /// Membership related message
    MembershipItem {
        #[serde(rename = "liveChatMembershipItemRenderer")]
        renderer: LiveChatMembershipItemRenderer,
    },
    /// Viewer engagement message (typically system messages)
    ViewerEngagementMessage {
        #[serde(rename = "liveChatViewerEngagementMessageRenderer")]
        renderer: LiveChatViewerEngagementMessageRenderer,
    },
    /// Announcement when someone purchases gift memberships
    SponsorshipsGiftPurchaseAnnouncement {
        #[serde(rename = "liveChatSponsorshipsGiftPurchaseAnnouncementRenderer")]
        renderer: LiveChatSponsorshipsGiftPurchaseAnnouncementRenderer,
    },
    /// Announcement when someone redeems a gifted membership
    SponsorshipsGiftRedemptionAnnouncement {
        #[serde(rename = "liveChatSponsorshipsGiftRedemptionAnnouncementRenderer")]
        renderer: LiveChatSponsorshipsGiftRedemptionAnnouncementRenderer,
    },
    /// Unknown chat item type
    Unknown(serde_json::Value),
}

impl ChatItem {
    /// Returns a string identifying the type of chat item
    pub fn get_type(&self) -> &'static str {
        match self {
            ChatItem::TextMessage { .. } => "textMessage",
            ChatItem::PaidMessage { .. } => "paidMessage",
            ChatItem::PaidSticker { .. } => "paidSticker",
            ChatItem::MembershipItem { .. } => "membershipItem",
            ChatItem::ViewerEngagementMessage { .. } => "viewerEngagementMessage",
            ChatItem::SponsorshipsGiftPurchaseAnnouncement { .. } => {
                "sponsorshipsGiftPurchaseAnnouncement"
            }
            ChatItem::SponsorshipsGiftRedemptionAnnouncement { .. } => {
                "sponsorshipsGiftRedemptionAnnouncement"
            }
            ChatItem::Unknown(_) => "unknown",
        }
    }

    /// Returns the viewer engagement message renderer if this is a viewer engagement message
    pub fn get_viewer_engagement_message(
        &self,
    ) -> Option<&LiveChatViewerEngagementMessageRenderer> {
        match self {
            ChatItem::ViewerEngagementMessage { renderer } => Some(renderer),
            _ => None,
        }
    }
}

/// Enum representing actions to add a chat item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AddChatItemAction {
    /// Message sent by a user
    UserMessage {
        /// Client ID associated with the message
        #[serde(rename = "clientId")]
        client_id: String,
        /// The chat item being added
        item: ChatItem,
        /// Optional tracking parameters
        #[serde(
            rename = "clickTrackingParams",
            skip_serializing_if = "Option::is_none"
        )]
        click_tracking_params: Option<String>,
    },
    /// Message sent by the system
    SystemMessage {
        /// The chat item being added
        item: ChatItem,
        /// Optional tracking parameters
        #[serde(
            rename = "clickTrackingParams",
            skip_serializing_if = "Option::is_none"
        )]
        click_tracking_params: Option<String>,
    },
}

impl AddChatItemAction {
    /// Returns true if this is a system message
    pub fn is_system_message(&self) -> bool {
        matches!(self, AddChatItemAction::SystemMessage { .. })
    }

    /// Gets the chat item contained in this action
    pub fn get_item(&self) -> &ChatItem {
        match self {
            AddChatItemAction::UserMessage { item, .. } => item,
            AddChatItemAction::SystemMessage { item, .. } => item,
        }
    }
}

/// Extract the continuation token for the next request from a response.
///
/// # Arguments
/// * `response` - The GetLiveChatResponse to extract the continuation from
///
/// # Returns
/// An Option containing the continuation string if available
pub fn get_next_continuation(response: &GetLiveChatResponse) -> Option<String> {
    response
        .continuation_contents
        .live_chat_continuation
        .continuation
        .as_ref()
        .map(|c| c.0.clone())
}

/// Count actions by their type in a collection of response entries.
///
/// # Arguments
/// * `entries` - A slice of ResponseEntry objects to analyze
///
/// # Returns
/// A HashMap with action types as keys and counts as values
pub fn count_actions_by_type(
    entries: &[ResponseEntry],
) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    let mut found_viewer_engagement_message = false;

    for entry in entries {
        for action in &entry
            .response
            .continuation_contents
            .live_chat_continuation
            .actions
        {
            // Skip tracking and moderation-related commands
            if let Action::Unknown(value) = action {
                if let Some(obj) = value.as_object() {
                    if obj.contains_key("clickTrackingParams")
                        && obj.contains_key("liveChatReportModerationStateCommand")
                    {
                        continue;
                    }
                }
            }

            let action_type = match action {
                Action::AddChatItem(wrapper) => {
                    // Determine if this is a system message or user message
                    let is_system = wrapper.action.is_system_message();
                    let prefix = if is_system { "system_" } else { "user_" };

                    // Get message type from the typed ChatItem
                    let item = wrapper.action.get_item();
                    let item_type = item.get_type();

                    // Only display message content for ViewerEngagementMessage system messages
                    if is_system
                        && item_type == "viewerEngagementMessage"
                        && !found_viewer_engagement_message
                    {
                        if let Some(renderer) = item.get_viewer_engagement_message() {
                            println!("Found ViewerEngagementMessage:");
                            if let Some(message_run) = renderer.message.runs.first() {
                                if let Some(text) = message_run.get_text() {
                                    println!("  Message: {}", text);
                                }
                            }
                            found_viewer_engagement_message = true;
                        }
                    }

                    format!("{}_{}", prefix, item_type)
                }
                Action::AddLiveChatTickerItem(..) => "addLiveChatTickerItemAction".to_string(),
                Action::RemoveChatItem(..) => "removeChatItemAction".to_string(),
                Action::Unknown(value) => {
                    // Get the first key of the object to identify unknown actions
                    if let Some(obj) = value.as_object() {
                        if let Some((key, _)) = obj.iter().next() {
                            format!("{}_unknown", key)
                        } else {
                            "unknown".to_string()
                        }
                    } else {
                        "unknown".to_string()
                    }
                }
            };

            *counts.entry(action_type).or_insert(0) += 1;
        }
    }

    counts
}

/// Count renderer types in a collection of response entries.
///
/// # Arguments
/// * `entries` - A slice of ResponseEntry objects to analyze
///
/// # Returns
/// A HashMap with renderer types as keys and counts as values
pub fn count_renderers_by_type(
    entries: &[ResponseEntry],
) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    let mut unknown_renderer_debug = false;
    let mut unknown_ticker_debug = false;

    // HashMap to track ticker item details for display
    let mut ticker_details = std::collections::HashMap::new();

    for entry in entries {
        for action in &entry
            .response
            .continuation_contents
            .live_chat_continuation
            .actions
        {
            match action {
                Action::AddChatItem(wrapper) => {
                    let is_system = wrapper.action.is_system_message();
                    let prefix = if is_system { "system" } else { "user" };
                    let item = wrapper.action.get_item();

                    match item {
                        ChatItem::TextMessage { renderer } => {
                            let key = format!("{}:text_message", prefix);
                            *counts.entry(key).or_insert(0) += 1;

                            // Count unique entries based on author name and start of message
                            if let Some(message_run) = renderer.message.runs.first() {
                                if let Some(text) = message_run.get_text() {
                                    // Create a UTF-8 safe preview of the message
                                    let msg_preview = if text.chars().count() > 20 {
                                        let mut result = String::new();
                                        for (i, c) in text.char_indices() {
                                            if i >= 20 {
                                                break;
                                            }
                                            result.push(c);
                                        }
                                        result + "..."
                                    } else {
                                        text.to_string()
                                    };

                                    let author_entry = format!(
                                        "  {}:{}: {}",
                                        prefix, renderer.author_name.simple_text, msg_preview
                                    );

                                    // Count unique entries per user
                                    if !counts.contains_key(&author_entry) {
                                        *counts.entry(author_entry).or_insert(0) += 1;
                                    }
                                } else if let Some(emoji) = message_run.get_emoji() {
                                    // For emoji-only messages
                                    let emoji_label =
                                        if let Some(accessibility) = &emoji.image.accessibility {
                                            &accessibility.accessibility_data.label
                                        } else {
                                            "emoji"
                                        };

                                    let author_entry = format!(
                                        "  {}:{}: [{}]",
                                        prefix, renderer.author_name.simple_text, emoji_label
                                    );

                                    if !counts.contains_key(&author_entry) {
                                        *counts.entry(author_entry).or_insert(0) += 1;
                                    }
                                }
                            }
                        }
                        ChatItem::PaidMessage { renderer } => {
                            let key = format!(
                                "{}:paid_message:{}",
                                prefix, renderer.purchase_amount_text.simple_text
                            );
                            *counts.entry(key).or_insert(0) += 1;
                        }
                        ChatItem::PaidSticker { renderer } => {
                            let key = format!(
                                "{}:paid_sticker:{}",
                                prefix, renderer.purchase_amount_text.simple_text
                            );
                            *counts.entry(key).or_insert(0) += 1;
                        }
                        ChatItem::MembershipItem { renderer } => {
                            let mut description = "membership";
                            if let Some(header) = &renderer.header_primary_text {
                                if let Some(run) = header.runs.first() {
                                    if let Some(text) = run.get_text() {
                                        description = text;
                                    }
                                }
                            }
                            let key = format!("{}:membership:{}", prefix, description);
                            *counts.entry(key).or_insert(0) += 1;
                        }
                        ChatItem::ViewerEngagementMessage { renderer } => {
                            if let Some(message_run) = renderer.message.runs.first() {
                                if let Some(text) = message_run.get_text() {
                                    let key = format!("{}:viewer_engagement:{}", prefix, text);
                                    *counts.entry(key).or_insert(0) += 1;
                                } else {
                                    let key = format!("{}:viewer_engagement", prefix);
                                    *counts.entry(key).or_insert(0) += 1;
                                }
                            } else {
                                let key = format!("{}:viewer_engagement", prefix);
                                *counts.entry(key).or_insert(0) += 1;
                            }
                        }
                        ChatItem::SponsorshipsGiftPurchaseAnnouncement { renderer } => {
                            if let Some(header_run) = renderer.header.runs.first() {
                                if let Some(text) = header_run.get_text() {
                                    let key = format!("{}:gift_purchase:{}", prefix, text);
                                    *counts.entry(key).or_insert(0) += 1;
                                } else {
                                    let key = format!("{}:gift_purchase", prefix);
                                    *counts.entry(key).or_insert(0) += 1;
                                }
                            } else {
                                let key = format!("{}:gift_purchase", prefix);
                                *counts.entry(key).or_insert(0) += 1;
                            }
                        }
                        ChatItem::SponsorshipsGiftRedemptionAnnouncement { renderer } => {
                            if let Some(message_run) = renderer.message.runs.first() {
                                if let Some(text) = message_run.get_text() {
                                    let key = format!("{}:gift_redemption:{}", prefix, text);
                                    *counts.entry(key).or_insert(0) += 1;
                                } else {
                                    let key = format!("{}:gift_redemption", prefix);
                                    *counts.entry(key).or_insert(0) += 1;
                                }
                            } else {
                                let key = format!("{}:gift_redemption", prefix);
                                *counts.entry(key).or_insert(0) += 1;
                            }
                        }
                        ChatItem::Unknown(value) => {
                            if let Some(obj) = value.as_object() {
                                if let Some((key, _)) = obj.iter().next() {
                                    let unknown_key = format!("{}:unknown:{}", prefix, key);
                                    *counts.entry(unknown_key).or_insert(0) += 1;

                                    // First unknown renderer type for debugging
                                    if !unknown_renderer_debug
                                        && key == "liveChatTextMessageRenderer"
                                    {
                                        println!("\n=== Debug: First Unknown Renderer ===");
                                        println!("Key: {}", key);
                                        println!(
                                            "This appears to be a liveChatTextMessageRenderer"
                                        );

                                        // Experiment: try to parse directly from serde_json
                                        if let Ok(renderer) =
                                            serde_json::from_value::<LiveChatTextMessageRenderer>(
                                                obj["liveChatTextMessageRenderer"].clone(),
                                            )
                                        {
                                            println!(
                                                "Successfully parsed as LiveChatTextMessageRenderer"
                                            );
                                            println!(
                                                "Author: {}",
                                                renderer.author_name.simple_text
                                            );
                                            println!(
                                                "Message runs: {}",
                                                renderer.message.runs.len()
                                            );

                                            // Investigate message content
                                            for (i, run) in renderer.message.runs.iter().enumerate()
                                            {
                                                println!("  Run {}: {:?}", i, run);
                                            }
                                        } else {
                                            println!(
                                                "Failed to parse as LiveChatTextMessageRenderer"
                                            );
                                            // Investigate message type key
                                            if let Some(message_obj) = obj
                                                ["liveChatTextMessageRenderer"]["message"]
                                                .as_object()
                                            {
                                                if let Some(runs_arr) = message_obj
                                                    .get("runs")
                                                    .and_then(|v| v.as_array())
                                                {
                                                    println!(
                                                        "Message runs found: {}",
                                                        runs_arr.len()
                                                    );

                                                    for (i, run) in runs_arr.iter().enumerate() {
                                                        println!(
                                                            "  Run {}: Available keys: {:?}",
                                                            i,
                                                            run.as_object().map(|o| o
                                                                .keys()
                                                                .collect::<Vec<_>>())
                                                        );

                                                        // Investigate emoji details
                                                        if let Some(emoji) = run.get("emoji") {
                                                            println!(
                                                                "    Emoji found: emojiId = {:?}",
                                                                emoji
                                                                    .get("emojiId")
                                                                    .and_then(|v| v.as_str())
                                                            );
                                                            println!(
                                                                "    Is custom emoji: {:?}",
                                                                emoji
                                                                    .get("isCustomEmoji")
                                                                    .and_then(|v| v.as_bool())
                                                            );
                                                        }

                                                        // Investigate text details
                                                        if let Some(text) = run.get("text") {
                                                            println!(
                                                                "    Text found: {:?}",
                                                                text.as_str()
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            println!(
                                                "Value: {}",
                                                serde_json::to_string_pretty(&value).unwrap()
                                            );
                                        }

                                        unknown_renderer_debug = true;
                                    }
                                } else {
                                    let unknown_key = format!("{}:unknown", prefix);
                                    *counts.entry(unknown_key).or_insert(0) += 1;
                                }
                            } else {
                                let unknown_key = format!("{}:unknown", prefix);
                                *counts.entry(unknown_key).or_insert(0) += 1;
                            }
                        }
                    }
                }
                Action::AddLiveChatTickerItem(wrapper) => {
                    let ticker_item = &wrapper.action.item;
                    let key = format!("ticker:{}", ticker_item.get_type());
                    *counts.entry(key.clone()).or_insert(0) += 1;

                    // Collect ticker item details
                    match ticker_item {
                        TickerItem::PaidMessage { renderer } => {
                            // Count by purchase amount
                            if let Some(author_username) = &renderer.author_username {
                                let username = &author_username.simple_text;
                                let detail_key = format!("ticker:paid_message:{}", username);
                                *ticker_details.entry(detail_key).or_insert(0) += 1;
                            }
                        }
                        TickerItem::PaidSticker { renderer: _ } => {
                            counts.entry("paidSticker".to_string()).or_insert(0);
                            *counts.entry("paidSticker".to_string()).or_insert(0) += 1;
                        }
                        TickerItem::Sponsor { renderer } => {
                            // Count membership details
                            if let Some(simple_text) = &renderer.detail_text.simple_text {
                                let detail_key = format!("ticker:sponsor:{}", simple_text);
                                *ticker_details.entry(detail_key).or_insert(0) += 1;
                            }
                        }
                        TickerItem::Unknown(value) => {
                            // Unknown ticker item for debugging
                            *ticker_details
                                .entry("ticker:unknown".to_string())
                                .or_insert(0) += 1;

                            // Debugging information
                            if !unknown_ticker_debug {
                                println!("\n=== Debug: First Unknown Ticker Item ===");

                                if let Some(obj) = value.as_object() {
                                    if let Some((key, _)) = obj.iter().next() {
                                        println!("Key: {}", key);

                                        // PaidMessage renderer
                                        if key == "liveChatTickerPaidMessageItemRenderer" {
                                            if let Ok(_renderer) = serde_json::from_value::<
                                                LiveChatTickerPaidMessageItemRenderer,
                                            >(
                                                obj["liveChatTickerPaidMessageItemRenderer"]
                                                    .clone(),
                                            ) {
                                                println!("Successfully parsed as LiveChatTickerPaidMessageItemRenderer");
                                            } else {
                                                println!("Failed to parse as LiveChatTickerPaidMessageItemRenderer");
                                                println!(
                                                    "Value: {}",
                                                    serde_json::to_string_pretty(&value).unwrap()
                                                );

                                                // Investigate fields
                                                if let Some(renderer_obj) =
                                                    obj.get(key).and_then(|v| v.as_object())
                                                {
                                                    println!(
                                                        "Available fields: {:?}",
                                                        renderer_obj.keys().collect::<Vec<_>>()
                                                    );
                                                }
                                            }
                                        }
                                        // SponsorItem renderer
                                        else if key == "liveChatTickerSponsorItemRenderer" {
                                            if let Ok(_renderer) = serde_json::from_value::<
                                                LiveChatTickerSponsorItemRenderer,
                                            >(
                                                obj["liveChatTickerSponsorItemRenderer"].clone(),
                                            ) {
                                                println!("Successfully parsed as LiveChatTickerSponsorItemRenderer");
                                            } else {
                                                println!("Failed to parse as LiveChatTickerSponsorItemRenderer");
                                                println!(
                                                    "Value: {}",
                                                    serde_json::to_string_pretty(&value).unwrap()
                                                );

                                                // Investigate fields
                                                if let Some(renderer_obj) =
                                                    obj.get(key).and_then(|v| v.as_object())
                                                {
                                                    println!(
                                                        "Available fields: {:?}",
                                                        renderer_obj.keys().collect::<Vec<_>>()
                                                    );
                                                }
                                            }
                                        }
                                        // PaidSticker renderer
                                        else if key == "liveChatTickerPaidStickerItemRenderer" {
                                            if let Ok(_renderer) = serde_json::from_value::<
                                                LiveChatTickerPaidStickerItemRenderer,
                                            >(
                                                obj["liveChatTickerPaidStickerItemRenderer"]
                                                    .clone(),
                                            ) {
                                                println!("Successfully parsed as LiveChatTickerPaidStickerItemRenderer");
                                            } else {
                                                println!("Failed to parse as LiveChatTickerPaidStickerItemRenderer");
                                                println!(
                                                    "Value: {}",
                                                    serde_json::to_string_pretty(&value).unwrap()
                                                );

                                                // Investigate fields
                                                if let Some(renderer_obj) =
                                                    obj.get(key).and_then(|v| v.as_object())
                                                {
                                                    println!(
                                                        "Available fields: {:?}",
                                                        renderer_obj.keys().collect::<Vec<_>>()
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }

                                unknown_ticker_debug = true;
                            }
                        }
                    }
                }
                Action::RemoveChatItem(_wrapper) => {
                    *counts.entry("remove_chat_item".to_string()).or_insert(0) += 1;
                }
                Action::Unknown(_) => {
                    *counts.entry("unknown_action".to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    // Add ticker item details to counts
    for (key, value) in ticker_details {
        if value > 0 {
            // Only add if there are counts
            counts.insert(key, value);
        }
    }

    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_file_path(filename: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("data");
        path.push(filename);
        path
    }

    #[test]
    // #[ignore] // テストデータファイルが不完全なため一時的に無効化
    fn test_get_next_continuation() {
        use crate::io::ndjson::parse_ndjson_file;

        let file_path = get_test_file_path("live_chat.ndjson");
        let entries = parse_ndjson_file(file_path.to_str().unwrap()).unwrap();

        // Test with the first entry that should have continuation data
        for entry in &entries {
            if let Some(continuation) = get_next_continuation(&entry.response) {
                assert!(!continuation.is_empty());
                break;
            }
        }
    }

    #[test]
    // #[ignore] // テストデータファイルが不完全なため一時的に無効化
    fn test_count_actions_by_type() {
        use crate::io::ndjson::parse_ndjson_file;

        let file_path = get_test_file_path("live_chat.ndjson");
        let entries = parse_ndjson_file(file_path.to_str().unwrap()).unwrap();

        let counts = count_actions_by_type(&entries);

        // テストデータには少なくとも何らかのアクションが含まれているはず
        assert!(!counts.is_empty());

        // 総アクション数が0より大きいことを確認
        let total_actions: usize = counts.values().sum();
        assert!(total_actions > 0);
    }

    #[test]
    fn test_message_run_text_extraction() {
        let text_run = MessageRun {
            text: Some("Hello World".to_string()),
            emoji: None,
        };

        assert_eq!(text_run.get_text(), Some("Hello World"));
        assert!(text_run.get_emoji().is_none());
    }

    #[test]
    fn test_message_run_emoji_extraction() {
        let emoji = Emoji {
            emoji_id: "test_emoji".to_string(),
            image: Image {
                thumbnails: vec![],
                accessibility: None,
            },
            search_terms: vec!["test".to_string()],
            shortcuts: vec![":test:".to_string()],
            is_custom_emoji: true,
        };

        let emoji_run = MessageRun {
            text: None,
            emoji: Some(emoji.clone()),
        };

        assert!(emoji_run.get_text().is_none());
        assert_eq!(emoji_run.get_emoji().unwrap().emoji_id, "test_emoji");
    }

    #[test]
    fn test_chat_item_type_detection() {
        let text_renderer = LiveChatTextMessageRenderer {
            id: "test_id".to_string(),
            message: Message { runs: vec![] },
            author_name: AuthorName {
                simple_text: "Test User".to_string(),
            },
            author_photo: AuthorPhoto { thumbnails: vec![] },
            timestamp_usec: "1234567890".to_string(),
            author_external_channel_id: "test_channel".to_string(),
            author_badges: vec![],
            tracking_params: None,
            context_menu_endpoint: None,
            context_menu_accessibility: None,
        };

        let chat_item = ChatItem::TextMessage {
            renderer: text_renderer,
        };
        assert_eq!(chat_item.get_type(), "textMessage");
    }

    #[test]
    fn test_add_chat_item_action_system_message_detection() {
        let chat_item = ChatItem::TextMessage {
            renderer: LiveChatTextMessageRenderer {
                id: "test_id".to_string(),
                message: Message { runs: vec![] },
                author_name: AuthorName {
                    simple_text: "Test User".to_string(),
                },
                author_photo: AuthorPhoto { thumbnails: vec![] },
                timestamp_usec: "1234567890".to_string(),
                author_external_channel_id: "test_channel".to_string(),
                author_badges: vec![],
                tracking_params: None,
                context_menu_endpoint: None,
                context_menu_accessibility: None,
            },
        };

        let system_action = AddChatItemAction::SystemMessage {
            item: chat_item.clone(),
            click_tracking_params: None,
        };

        let user_action = AddChatItemAction::UserMessage {
            client_id: "test_client".to_string(),
            item: chat_item,
            click_tracking_params: None,
        };

        assert!(system_action.is_system_message());
        assert!(!user_action.is_system_message());
    }

    #[test]
    fn test_ticker_item_type_detection() {
        let paid_message_renderer = LiveChatTickerPaidMessageItemRenderer {
            id: "test_id".to_string(),
            author_external_channel_id: "test_channel".to_string(),
            author_photo: AuthorPhoto { thumbnails: vec![] },
            author_username: None,
            start_background_color: 0xFF0000,
            end_background_color: 0x00FF00,
            duration_sec: 30,
            full_duration_sec: 60,
            amount_text_color: 0x000000,
            tracking_params: "test_params".to_string(),
            animation_origin: None,
            dynamic_state_data: None,
            show_item_endpoint: ShowItemEndpoint {
                show_live_chat_item_endpoint: ShowLiveChatItemEndpoint {
                    renderer: serde_json::Value::Null,
                },
            },
            open_engagement_panel_command: None,
        };

        let ticker_item = TickerItem::PaidMessage {
            renderer: paid_message_renderer,
        };
        assert_eq!(ticker_item.get_type(), "paidMessage");
    }

    #[test]
    fn test_continuation_wrapper() {
        let continuation = Continuation("test_continuation_token".to_string());
        assert_eq!(continuation.0, "test_continuation_token");
    }

    #[test]
    fn test_simple_text_structure() {
        let simple_text = SimpleText {
            simple_text: "Test message".to_string(),
        };
        assert_eq!(simple_text.simple_text, "Test message");
    }

    #[test]
    fn test_accessibility_data_structure() {
        let accessibility_data = AccessibilityData {
            label: "Test accessibility label".to_string(),
        };

        let accessibility = Accessibility { accessibility_data };

        assert_eq!(
            accessibility.accessibility_data.label,
            "Test accessibility label"
        );
    }

    #[test]
    fn test_count_renderers_by_type() {
        use crate::io::ndjson::parse_ndjson_file;

        let file_path = get_test_file_path("live_chat.ndjson");
        let entries = parse_ndjson_file(file_path.to_str().unwrap()).unwrap();

        let counts = count_renderers_by_type(&entries);

        // テストデータには少なくとも何らかのレンダラーが含まれているはず
        assert!(!counts.is_empty());

        // 総レンダラー数が0より大きいことを確認
        let total_renderers: usize = counts.values().sum();
        assert!(total_renderers > 0);
    }
}
