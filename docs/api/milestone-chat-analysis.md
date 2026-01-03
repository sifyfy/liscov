# マイルストーンチャット (Milestone Chat) 分析レポート

## 概要

YouTubeライブチャットにおけるマイルストーンチャット（`liveChatMembershipItemRenderer`）の生レスポンス構造を分析し、liscovでの対応方針をまとめる。

## 生レスポンスのサンプル

### サンプル1: 絵文字を含むマイルストーンチャット (13ヶ月記念)

**ファイル**: `<XDG_DATA_DIR>/raw_responses_YYYYMMDD_HHMMSS.ndjson`
**タイムスタンプ**: 1767338028

```json
{
  "liveChatMembershipItemRenderer": {
    "authorBadges": [
      {
        "liveChatAuthorBadgeRenderer": {
          "accessibility": {
            "accessibilityData": {
              "label": "Member (1 year)"
            }
          },
          "customThumbnail": {
            "thumbnails": [
              {
                "height": 16,
                "url": "https://yt3.ggpht.com/BADGE_THUMBNAIL_ID=s16-c-k",
                "width": 16
              },
              {
                "height": 32,
                "url": "https://yt3.ggpht.com/BADGE_THUMBNAIL_ID=s32-c-k",
                "width": 32
              }
            ]
          },
          "tooltip": "Member (1 year)"
        }
      }
    ],
    "authorExternalChannelId": "UCxxxxxxxxxxxxxxxxxxxxxxxxx",
    "authorName": {
      "simpleText": "@SampleUser1"
    },
    "authorPhoto": {
      "thumbnails": [
        {
          "height": 32,
          "url": "https://yt4.ggpht.com/AUTHOR_PHOTO_ID=s32-c-k-c0x00ffffff-no-rj",
          "width": 32
        },
        {
          "height": 64,
          "url": "https://yt4.ggpht.com/AUTHOR_PHOTO_ID=s64-c-k-c0x00ffffff-no-rj",
          "width": 64
        }
      ]
    },
    "headerPrimaryText": {
      "runs": [
        { "text": "Member for " },
        { "text": "13" },
        { "text": " months" }
      ]
    },
    "headerSubtext": {
      "simpleText": "コントリビューター"
    },
    "id": "DUMMY_MESSAGE_ID_001",
    "message": {
      "runs": [
        {
          "emoji": {
            "emojiId": "👊",
            "image": {
              "accessibility": {
                "accessibilityData": {
                  "label": "👊"
                }
              },
              "thumbnails": [
                {
                  "url": "https://fonts.gstatic.com/s/e/notoemoji/15.1/1f44a/72.png"
                }
              ]
            },
            "searchTerms": ["oncoming", "fist", "facepunch", "punch"],
            "shortcuts": [":oncoming_fist:", ":facepunch:", ":punch:"],
            "supportsSkinTone": true,
            "variantIds": ["👊", "👊🏻", "👊🏼", "👊🏽", "👊🏾", "👊🏿"]
          }
        }
      ]
    },
    "timestampUsec": "1767338026816689",
    "trackingParams": "DUMMY_TRACKING_PARAMS_001"
  }
}
```

### サンプル2: 複数の絵文字とテキストを含むマイルストーンチャット (1ヶ月記念)

**タイムスタンプ**: 1767338059

```json
{
  "liveChatMembershipItemRenderer": {
    "authorBadges": [
      {
        "liveChatAuthorBadgeRenderer": {
          "accessibility": {
            "accessibilityData": {
              "label": "Member (1 month)"
            }
          },
          "customThumbnail": {
            "thumbnails": [
              {
                "height": 16,
                "url": "https://yt3.ggpht.com/BADGE_THUMBNAIL_ID_2=s16-c-k",
                "width": 16
              },
              {
                "height": 32,
                "url": "https://yt3.ggpht.com/BADGE_THUMBNAIL_ID_2=s32-c-k",
                "width": 32
              }
            ]
          },
          "tooltip": "Member (1 month)"
        }
      }
    ],
    "authorExternalChannelId": "UCyyyyyyyyyyyyyyyyyyyyyyyyy",
    "authorName": {
      "simpleText": "@SampleUser2"
    },
    "authorPhoto": {
      "thumbnails": [
        {
          "height": 32,
          "url": "https://yt4.ggpht.com/AUTHOR_PHOTO_ID_2=s32-c-k-c0x00ffffff-no-rj",
          "width": 32
        },
        {
          "height": 64,
          "url": "https://yt4.ggpht.com/AUTHOR_PHOTO_ID_2=s64-c-k-c0x00ffffff-no-rj",
          "width": 64
        }
      ]
    },
    "headerPrimaryText": {
      "runs": [
        { "text": "Member for 1 month" }
      ]
    },
    "headerSubtext": {
      "simpleText": "コントリビューター"
    },
    "id": "DUMMY_MESSAGE_ID_002",
    "message": {
      "runs": [
        {
          "emoji": {
            "emojiId": "👊",
            "image": {
              "accessibility": {
                "accessibilityData": {
                  "label": "👊"
                }
              },
              "thumbnails": [
                {
                  "url": "https://fonts.gstatic.com/s/e/notoemoji/15.1/1f44a/72.png"
                }
              ]
            },
            "searchTerms": ["oncoming", "fist", "facepunch", "punch"],
            "shortcuts": [":oncoming_fist:", ":facepunch:", ":punch:"],
            "supportsSkinTone": true,
            "variantIds": ["👊", "👊🏻", "👊🏼", "👊🏽", "👊🏾", "👊🏿"]
          }
        },
        {
          "emoji": {
            "emojiId": "👊",
            "image": {...},
            ...
          }
        },
        {
          "emoji": {
            "emojiId": "👊",
            "image": {...},
            ...
          }
        },
        {
          "emoji": {
            "emojiId": "👊",
            "image": {...},
            ...
          }
        },
        {
          "text": "サンプルテキストメッセージ"
        }
      ]
    },
    "timestampUsec": "1767338058096889",
    "trackingParams": "DUMMY_TRACKING_PARAMS_002"
  }
}
```

### サンプル3: 新規メンバー登録 (Welcome)

```json
{
  "liveChatMembershipItemRenderer": {
    "authorBadges": [...],
    "authorExternalChannelId": "UCzzzzzzzzzzzzzzzzzzzzzzzzz",
    "authorName": {
      "simpleText": "@SampleUser3"
    },
    "authorPhoto": {...},
    "headerSubtext": {
      "runs": [
        { "text": "Welcome to ", "emoji": null },
        { "text": "サンプルチャンネル", "emoji": null },
        { "text": "!", "emoji": null }
      ]
    },
    "id": "DUMMY_MESSAGE_ID_003",
    "message": null,
    "timestampUsec": "1767084627649165",
    "trackingParams": "DUMMY_TRACKING_PARAMS_003"
  }
}
```

## レスポンス構造の特徴

### 1. `headerSubtext` のフォーマットバリエーション

`headerSubtext`には2種類のフォーマットが存在する:

| フォーマット | 例 | 用途 |
|-------------|-----|------|
| `simpleText` | `{"simpleText":"コントリビューター"}` | メンバーシップティア名 |
| `runs` | `{"runs":[{"text":"Welcome to "},{"text":"チャンネル名"},{"text":"!"}]}` | 新規メンバー登録メッセージ |

### 2. `headerPrimaryText` の存在

- **マイルストーン記念の場合**: 存在する（例: `"Member for 13 months"`）
- **新規メンバー登録の場合**: 存在しない（`null` または欠落）

### 3. `message` フィールド

- **ユーザーがメッセージを入力した場合**: `runs`配列を持つ
- **メッセージなしの場合**: `null`

`message.runs`の構造:
```json
{
  "runs": [
    { "emoji": {...} },      // 絵文字のみ
    { "text": "テキスト" },   // テキストのみ
    { "text": null, "emoji": {...} }  // 明示的なnull
  ]
}
```

## 現在のliscov実装の問題点

### 問題1: `headerSubtext`の`simpleText`形式が未対応

**現在の`Message`構造体** (`src/api/innertube/get_live_chat.rs`):
```rust
pub struct Message {
    pub runs: Vec<MessageRun>,
}
```

`simpleText`フィールドがないため、`{"simpleText":"コントリビューター"}`形式のデシリアライズに問題が生じる可能性がある。

### 問題2: `extract_message_text`が絵文字を無視

**現在の実装** (`src/gui/models.rs`):
```rust
fn extract_message_text(runs: &[MessageRun]) -> String {
    runs.iter()
        .filter_map(|run| run.get_text().map(|s| s.to_string()))
        .collect::<Vec<_>>()
        .join("")
}
```

この関数はテキストのみを抽出し、絵文字は無視される。`content`フィールドに絵文字が含まれない。

### 問題3: マイルストーンチャットのruns構築は正常

**現在の実装** (`src/gui/models.rs`):
```rust
let runs = if let Some(msg) = &renderer.message {
    msg.runs
        .iter()
        .filter_map(|run| {
            if let Some(text) = run.get_text() {
                Some(MessageRun::Text { content: text.to_string() })
            } else if let Some(emoji) = run.get_emoji() {
                // 絵文字も正しく処理している
                Some(MessageRun::Emoji { ... })
            } else {
                None
            }
        })
        .collect()
} else {
    Vec::new()
};
```

`runs`の構築は絵文字を正しく処理している。

## 修正方針

### 方針1: `Message`構造体の拡張

`simpleText`形式にも対応するため、`Message`を以下のように変更:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub runs: Vec<MessageRun>,
    #[serde(rename = "simpleText", skip_serializing_if = "Option::is_none")]
    pub simple_text: Option<String>,
}

impl Message {
    /// runsまたはsimpleTextからテキストを取得
    pub fn get_text(&self) -> String {
        if let Some(text) = &self.simple_text {
            text.clone()
        } else {
            self.runs.iter()
                .filter_map(|run| run.get_text().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join("")
        }
    }
}
```

### 方針2: `extract_message_text`の修正（オプション）

絵文字のaltテキストを含める場合:

```rust
fn extract_message_text(runs: &[MessageRun]) -> String {
    runs.iter()
        .filter_map(|run| {
            if let Some(text) = run.get_text() {
                Some(text.to_string())
            } else if let Some(emoji) = run.get_emoji() {
                // 絵文字のaltテキストまたはIDを含める
                Some(emoji.emoji_id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("")
}
```

### 方針3: UIでの表示確認

現在の`runs`構築は正しく動作しているため、UI側で`runs`を正しく描画しているか確認する。

## 統計情報

**分析対象ファイル**: ログファイル (約104MB)

| レンダラータイプ | 件数 |
|-----------------|------|
| `liveChatTextMessageRenderer` | 34,804 |
| `liveChatAuthorBadgeRenderer` | 20,916 |
| `liveChatMembershipItemRenderer` | 59 |
| `liveChatPaidMessageRenderer` | 281 |
| `liveChatPaidStickerRenderer` | 6 |
| `liveChatPlaceholderItemRenderer` | 186 |
| `liveChatSponsorshipsGiftPurchaseAnnouncementRenderer` | 5 |
| `liveChatSponsorshipsGiftRedemptionAnnouncementRenderer` | 7 |
| `liveChatSponsorshipsHeaderRenderer` | 5 |
| `liveChatTickerPaidMessageItemRenderer` | 123 |
| `liveChatTickerSponsorItemRenderer` | 30 |

## 優先度

1. **高**: `Message`構造体の`simpleText`対応
2. **中**: `extract_message_text`の絵文字対応（オプション）
3. **低**: UI表示の検証

## 参考情報

- **ログファイル場所**: `<XDG_DATA_DIR>/`
- **設定ファイル**: `<XDG_CONFIG_DIR>/config.toml`
- **関連コード**:
  - `src/api/innertube/get_live_chat.rs` - APIレスポンス構造体
  - `src/gui/models.rs` - GUI用メッセージ変換
