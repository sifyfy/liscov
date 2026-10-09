# チャット接続・モニタリング機能

## 目的（Why）

配信者がYouTube Live配信のチャットをリアルタイムで監視し、視聴者への対応（スーパーチャット確認、初見さん歓迎、メンバー識別等）を行うためのアプリケーションの核心機能。複数配信への同時接続により、コラボ配信や複数チャンネル管理にも対応する。

## 振る舞い（What）

### 接続

| 操作 | 結果 |
|-----|------|
| URL入力 + 「開始」クリック | YouTubeページをフェッチし、APIキー・Continuation Tokenを抽出。チャット監視タスクを起動。ConnectionResultを返却 |
| 同一URLに再接続 | 新規セッションとして接続（メッセージ復元なし）。配信内コメント数カウンタはDBから復元 |
| メンバー限定配信に認証なしで接続 | 接続失敗（Member Only エラー） |
| 無効なURLを入力 | 接続失敗（Invalid URL エラー） |

### メッセージ受信

| 状況 | 結果 |
|------|------|
| 新着メッセージ受信 | DBに保存、視聴者プロフィール更新、フロントエンドにTauriイベント発行 |
| 同じ接続で既に受け取った message_id が再び届いた | 捨てる。DB保存・配信内コメント数・GUI・WebSocket・TTS のいずれにも流さない |
| 別の接続で受け取った message_id が届いた | 新着として扱う（重複の判定は接続ごと） |
| 同じ配信に再接続し、前の接続で受け取ったコメントが同じ id で届いた | 別の接続のメッセージとして並べる（同じコメントが2つ並ぶ）。一覧・視聴者パネルのキーや選択の判定も `connection_id:message_id` で行い、message_id だけでは扱わない |
| 取得に失敗（タイムアウト・ネットワークエラー・HTTPエラー・JSONでない応答） | warnログを出力し、次のポーリングで再試行。続けて失敗したら待ちを延ばし、15回続いたら切断する（「取得に失敗したとき」） |
| 取得の応答を待っている間に切断 | 応答を待たずに止まり、セッションを閉じる |
| DB保存エラー | warnログを出力し、メッセージ処理は継続 |

### 初見さん判定

| 条件 | 判定 |
|------|------|
| その配信者チャンネルで初めてコメントした視聴者（`viewer_streams`の最古video_idが現在のvideo_id） | `is_first_time_viewer = true` |
| 過去に別の配信でコメント済み | `is_first_time_viewer = false` |
| 配信者Aで初見 + 配信者Bでは常連 | 配信者ごとに独立判定 |
| 視聴者を特定できないギフト（`channel_id == ""`） | `is_first_time_viewer = false` |

### ギフト

視聴者がジュエルで送るギフト（メンバーシップギフトとは別物）。InnerTube では `giftMessageViewModel` で届き、channel_id とタイムスタンプが付かない。

**パース:**

| 入力 | 結果 |
|------|------|
| `text` = `sent Press F for 10 Jewels` | gift_name = `Press F`、jewel_count = `Some(10)` |
| `text` = `sent Press F for 1 Jewel` | gift_name = `Press F`、jewel_count = `Some(1)` |
| `text` = `sent Hiding` | gift_name = `Hiding`、jewel_count = `None` |
| `text` が `sent ` で始まらない、または空。`giftImageA11yLabel` = `@x sent a gift, Hiding` | gift_name = `Hiding`、jewel_count = `None` |
| `text` からもラベルからも名前が取れない | gift_name = `ギフト`、jewel_count = `None`。warn ログを出す |
| `authorName` = `@viewer-a1b `（末尾に空白） | author = `@viewer-a1b` |
| `giftImage.sources[].url` = `//www.gstatic.com/...=w480-h480` | gift_image_url = `https://www.gstatic.com/...=w480-h480`（幅が最小の source を使う） |
| `giftImage` が無い | gift_image_url = `None` |
| タイムスタンプ | 受信した時刻 |
| バッジ・メンバー情報 | 無し（is_member = false、badges は空） |

ジュエル数は配信者本人としてログインした接続でのみ本文に入ると見られる（観測は3件: ログインあり2件はジュエル数あり、ログインなし1件は無し）。

**視聴者の特定（handle → channel_id）:** 次の順に探し、最初に見つかったものを使う。

1. その接続で受信済みのメッセージの author → channel_id（同じ handle は最後に見たもの）
2. DB: その配信者の `viewer_profiles` で `display_name` が handle と完全一致するもの。一致が2件以上なら特定しない

| 状況（配信者 UCown） | 結果 |
|------|------|
| 同じ接続で `@viewer-a1b`（UCa1b）がコメント済み | channel_id = UCa1b |
| 接続中は未発言、DB の UCown 配下に `@viewer-a1b` → UCa1b | channel_id = UCa1b |
| DB の UCown 配下に `@viewer-a1b` が2件（UCa1b・UCzzz） | 特定しない |
| どこにも無い | 特定しない |
| 配信者の channel_id が分からない接続 | 1 だけで探す |

| 特定の結果 | 扱い |
|-----------|------|
| 特定できた | 通常のメッセージと同じ（viewer_profiles・viewer_streams・初見判定・配信内コメント回数・視聴者情報パネル） |
| 特定できない | `channel_id = ""` で保存・表示。viewer_profiles・viewer_streams は更新せず、初見判定は false、配信内コメント回数は数えない（`None`）。視聴者情報パネルは開かない |

保存済みのギフトの channel_id を後から埋めることはしない。

**ジュエル数を取れない接続の知らせ:**

| 状況 | 結果 |
|------|------|
| jewel_count = None のギフトを受信 | 接続一覧のその接続に注記「ジュエル数を取得できません。配信者本人としてログインした接続でのみ取得できます」を出す |
| jewel_count がある ギフトだけを受信 | 注記を出さない |
| 切断・再接続・F5リロード | 注記は消える。再びジュエル数の無いギフトを受けたら出る |

ダイアログや通知は出さない（配信中の操作を妨げない）。

### ライブリアクション

視聴者がチャット欄から送る絵文字リアクション（❤ 🎉 😄 💯 😳 など）。チャットメッセージではなく、送り主は分からない。InnerTube では応答の `frameworkUpdates.entityBatchUpdate.mutations[].payload.emojiFountainDataEntity` で、前回の更新からの差分として届く（変化が無いときは届かない）。

**パース（1つの `emojiFountainDataEntity` → 1つの ReactionUpdate）:** `reactionBuckets[]`（1秒単位の区切り）を合算する。

| 入力 | 結果 |
|------|------|
| `updateTimeUsec` = `1790422357025983`、バケット2つ（どちらも `duration.seconds` = `1`、`reactionsData` = `[{"unicodeEmojiId":"🎉","reactionCount":4}]`） | update_time_usec = 1790422357025983、duration_seconds = 2、counts = {🎉: 8}、total = 8 |
| バケット1つ（`reactionsData` = `[{"unicodeEmojiId":"❤","reactionCount":3},{"unicodeEmojiId":"😄","reactionCount":1}]`）とバケット1つ（`totalReactions` = 0、`reactionsData` 無し） | duration_seconds = 2、counts = {❤: 3, 😄: 1}、total = 4 |
| すべてのバケットが `totalReactions` = 0 | 捨てる（保存もイベントも出さない） |
| `unicodeEmojiId` か `reactionCount` が無い要素 | その要素だけ無視する |
| `duration` が無いバケット | 1秒として数える（観測ではすべて1秒） |
| `updateTimeUsec` が無い | 捨てる。warn ログを出す |
| 同じ接続で、update_time_usec が前回受けた値以下 | 捨てる（同じ更新の再送） |

- total は counts の合計（`totalReactions` は使わない。件数の出どころを1つにする）
- `intensityScore` は使わない（観測では件数ありで 0.75、0件で 1 の2値だけで情報が無い）
- 1回の更新の中のバケットの時間順は応答から判別できないため、1秒ごとの時刻は付けない。分析の細かさは更新ごと（観測では間隔の中央値1.6秒）

**扱い:**

| 項目 | 扱い |
|------|------|
| DB | `reactions` テーブルに更新×絵文字ごとに1行（[08_database.md](08_database.md)） |
| チャット欄 | メッセージとしては流さない。リアクションメーターに出す |
| 読み上げ | しない |
| WebSocket | `Reaction` を配信する（[03_websocket.md](03_websocket.md)） |
| 視聴者・初見判定・配信内コメント回数 | 関係しない（送り主が分からない） |

**リアクションメーター:** 接続ごとに1行。チャット欄の上に出す。

```
■ 配信者名A  ❤ 430  🎉 418  😄 94  💯 56  😳 1   勢い [■■■■□□□□] 38/分
```

| 項目 | 仕様 |
|------|------|
| 累計 | 配信（video_id）単位の絵文字別件数。多い順に並べる。同じ配信に再接続したら、DB から続きを数える |
| 勢い | 直近60秒の件数（/分）。チャット率と同じ「1分あたり」に揃える。60秒の判定は update_time_usec と現在時刻で行い、1秒ごとに表示を更新する（リアクションが止まれば0に下がる） |
| 勢いのバー | その接続で表示を始めてからの勢いの最大値を満タンとする |
| 表示条件 | その接続で1件以上のリアクションがあるとき。無い接続の行は出さない |
| 配信元の区別 | 左端に配信元色（メッセージの色ラインと同じ色）と配信者名 |

| 状況 | 結果 |
|------|------|
| 🎉4（2秒）→ 30秒後に ❤3 | 累計 ❤ 3・🎉 4、勢い 7/分 |
| 最後の更新から60秒を超えた | 累計はそのまま、勢い 0/分 |
| F5リロード | 累計と直近60秒の件数を `get_connection_reactions` で読み直す。バーの最大値は読み直した勢いから数え直す |
| 同じ配信に再接続 | 累計は DB の続きから。勢いは直近60秒の DB の記録から |
| 切断 | その接続の行を消す |

### 多接続

| 操作 | 結果 |
|------|------|
| 2つ目以降のURLで「開始」クリック | 追加接続。全接続のメッセージをグローバルバッファに統合表示 |
| 接続済みと接続中を合わせて32件あるときに「開始」 | 接続を拒否する（「同時接続数の上限（32）に達しています」）。同時に「開始」が続いても32件を超えない。切断・接続失敗で枠が空く |
| 「全切断」クリック | 接続済みの全接続を一括切断。接続中（`connecting`）のものは残り、成立したら一覧に出る。切断の途中で成立した接続も止めずに残す（押した時点の接続だけを切る） |
| 接続中（`connecting`）のエントリ | 切断ボタンは押せない。接続が成立するか失敗するまで待つ（接続処理には HTTP タイムアウトがあるので待ち続けない） |
| 接続が成立した | URL 入力欄を空にする。ただし欄の内容がその接続の URL から変わっていたら残す |
| F5リロード | バックエンド接続は維持。`get_connections`で状態を復元し、新着メッセージを正常受信 |
| コメントをクリックして視聴者パネルを開く | そのコメントが届いた接続の配信者のもとで視聴者を引き、読み仮名・メモもその配信者のもとに保存する（TTS が読み仮名を引く配信者と同じ）。最初の接続の配信者ではない（「視聴者情報パネル」の「どの配信者のもとで開くか」） |

### 配信中の画面

デザインの前提は [docs/design/brief.md](../design/brief.md)、見た目の値は [docs/design/tokens.md](../design/tokens.md)。
配信中 (特にゲーム中) は配信者が liscov を操作できないので、**配信中の目的は操作ゼロで達成できる**ことを基準にする。
試作は `src/routes/design-proto/c` (方向性 C「流れ型」、2026-10-08 に選定)。

**構成 (上から):**

```
┌───────────────────────────────────────────┐
│ 💬 👥 📊 ⚙ │ ●配信者A ×  ●あおば ×  +   ∨ │ ← 上の行 (タブ・接続の札・操作パネルの開閉)
├───────────────────────────────────────────┤
│ 操作パネル                                │ ← 開いたときだけ (「操作パネル」)
├───────────────────────────────────────────┤
│ この配信のスパチャ等 (時刻順)       閉じる │ ← お礼の一覧。開いたときだけ。高さの 1/2 まで
├───────────────────────────────────────────┤
│                                           │
│ コメントの流れ (最新が下)                 │ ← 残りの高さすべて
│                                           │
├───────────────────────────────────────────┤
│ 流れた新規さん 3                          │ ← あるときだけ。高さの 1/8 まで
├───────────────────────────────────────────┤
│ 流れたスパチャ等 5                        │ ← あるときだけ。高さの 1/8 まで
├───────────────────────────────────────────┤
│ この配信のスパチャ等の一覧 6              │ ← 自分の配信のスパチャ等があるときだけ
├───────────────────────────────────────────┤
│ ● 1,820 件  48/分  312 人   ❤ 128 🎉 31  │ ← 状態の行
└───────────────────────────────────────────┘
```

- 接続・切断・フィルタ・表示の設定などの操作は、操作パネルにまとめ、配信中は隠しておける
- 配信中に固定で場所をとる UI (上の行と状態の行) は、高さの 15% 以下 (ブリーフ §3)

**上の行と操作パネル:** (方向性の試作は `src/routes/design-proto/frame` の C、2026-10-09 に選定。名前はシフィさん)

| 状況 | 結果 |
|------|------|
| いつも | 上の行に、画面のタブ (アイコン。名前はヒントと読み上げ用の名前で出す)、接続の札、配信をつなぐ「+」、操作パネルを開け閉めする矢印 (右端、∨ / ∧) を出す |
| 接続の札 | 接続ごとに「状態の点 + 配信者名 + 切断 (×)」。点は接続中なら success の色、つなぎ直し中・失敗は error の色。主の配信の名前は太字 |
| 矢印を押す・「+」を押す | 操作パネルが開く (「+」は配信の URL の入力に移る)。もう一度矢印を押すと閉じる |
| 操作パネルの中身 | 接続の一覧 (配信者名・配信タイトル・ジュエル数の注意・切断・全切断)、配信をつなぐ (URL・すべてのコメントを取るか・つなぐ)、表示の設定 (文字の大きさ・時刻・表示件数・種類のフィルタ・検索・コメントを消す)、ログインの状態、「つないだら操作パネルを隠す」の設定 |
| 配信につながった (接続に成功した) | 「つないだら操作パネルを隠す」が有効 (既定) なら、操作パネルを閉じる。無効なら開いたまま |
| 「つないだら操作パネルを隠す」を無効にした | 操作パネルは手で閉じるまで開いたまま (コラボで続けて接続する人、開いたままが好みの人のため。シフィさん、2026-10-09) |
| 未接続 | 操作パネルの開閉によらず、流れの場所に配信の URL の入力を出す (配信の準備なので操作してよい) |
| フィルタ・表示件数で一部を隠している | 操作パネルを閉じていても、状態の行に「表示中 N / 全 M 件」を出す (隠していることを忘れない) |
| 540 幅 | ボタンの文字を縦に折らない。中身は折り返して並べ、省略しない |

**主の配信と相手の配信:**

この節の「自分の配信」は**主の配信**のこと。主の配信は最初の接続で決め、配信者本人のチャンネルかどうかは問わない
(テストで他の人の配信に接続することもある。シフィさん、2026-10-09)。

| 状況 | 結果 |
|------|------|
| 接続が 1 つ | すべて主の配信のもの (他の人の配信に接続していても同じ) |
| 接続が 2 つ以上 | 最初の接続 (`broadcasterChannelId` と同じ) が主の配信。それ以外は相手の配信 |
| 主の配信を切断し、他の接続が残った | 残った接続のうち、いちばん先に接続したものが主の配信になる |

**優先順位と強調** (ブリーフ §2。判定は `src/lib/design/priority.ts` の `messagePriority`):

| コメント | 優先順位 | 流れの中での表し方 |
|------|------|------|
| 自分の配信のスパチャ・スーパーステッカー・ギフト・メンバーシップ (ギフト・マイルストーンを含む) | 1 | 面 (スパチャ等は YouTube のティアの色、メンバーシップ系は member の色) |
| 新規さんの、その配信での初回 (`is_first_time_viewer` かつ `#1`) | 2 | 薄い背景 + 塗りの印「初見」 |
| 新規さんの 2 回目以降 | 3 | 枠だけの印「初見」(背景なし) |
| 常連の、その配信での初回 (`#1`) | 4 | 名前の前の小さい点 |
| それ以外 (配信内コメント回数が分からないものを含む) | 5 | 印なし (標準) |
| 相手の配信のスパチャ等 | 5 | 標準の行に金額 (またはメンバーシップの種類) の枠つきの札を添える。お礼の対象ではない |
| 相手の配信の新規さん・常連 | 自分の配信と同じ (2〜5) | 同じ表し方 + 配信元の札 |

**1 行の形:**

| 状況 | 結果 |
|------|------|
| 通常のコメント | 名前と本文を同じ行に続ける。その後ろに `#N` と時刻を控えめに |
| 長い本文・長い名前 | 折り返して全文を出す。流れ・流れたもの・お礼の一覧のどこでも、省略 (`…`) しない (省略された文字を読むには操作が要る) |
| 相手の配信のコメント | 名前の前に配信者名の札 (色ではなく文字で見分ける。540px 幅でも読める)。強さは変えない |
| 読み仮名が登録されている視聴者 | 名前の横に小さく出す |
| メンバーのコメント | 名前を緑の文字にし、メンバーバッジの画像を名前の横に出す (YouTube と同じ。背景・枠は付けない)。バッジの画像が無ければ「メンバー」の文字の札。新規さんの初回の行では名前は通常の色 (薄い背景の上で緑が読みにくいため) |
| モデレーター・認証済み | 名前の横に小さい印 (🔧・✓) |
| 時刻 | 表示の設定で消せる (今と同じ) |
| 文字の大きさ | 表示の設定 (10〜24px) を流れ・欄・一覧のコメントの文字に使う (今と同じ) |

**流れたもの (流れて画面の外に出た優先順位 1・2):**

| 状況 | 結果 |
|------|------|
| 自分の配信のスパチャ等が、流れの画面の外 (上) に出た | 「流れたスパチャ等」の欄の先頭に出る |
| 新規さんの初回コメント (相手の配信のものを含む) が画面の外に出た | 「流れた新規さん」の欄の先頭に出る |
| 欄の中の並び | 新しいものが上。新しい 2 件は全文の行 (時刻・金額・名前・本文)、それより前は「● 金額 名前」(新規さんは「初見 名前」) の短い札を折り返して並べる |
| 手でスクロールして戻り、そのコメントがまた画面に見えた | 欄から外れる (見えているものを二重に出さない) |
| 時間がたった | 消えない (最後のお礼のために残す) |
| 欄の中身が高さの上限 (1/8) を超えた | その欄の中でスクロールする。もう一方の欄の高さは変わらない |
| 相手の配信のスパチャ等 | どちらの欄にも入らない |
| フィルタで隠しているもの | 欄に入らない (流れに出ていないので、流れてもいない)。お礼の一覧はフィルタによらない |
| 「クリア」 | 欄も空になる |

例 (640×1080): ¥10,000 の赤スパのあとにコメントが 40 件来て流れの画面から外れた → 「流れたスパチャ等」の欄の先頭に `21:17 ¥10,000 ねむいペンギン 遅れてきました！…` が全文で出る。

**配信の最後のお礼:**

| 状況 | 結果 |
|------|------|
| 「この配信のスパチャ等の一覧」を押す (1 手) | 上に一覧が開く。下にコメントの流れが残り、最新のコメントも見える |
| 一覧の中身 | 自分の配信の優先順位 1 を時刻順 (古いものが上)。画面のメッセージ一覧にあるもの (「クリア」すると消える) |
| 一覧が開いている間の「流れたスパチャ等」の欄 | 出さない (一覧に含まれる)。「流れた新規さん」の欄は出したまま (一覧に含まれず、反応は時間が勝負) |
| 「閉じる」を押す | 一覧が閉じる |

**自動スクロール:**

| 状況 | 結果 |
|------|------|
| 新着が来た (最新を表示中) | 最新までスクロールする |
| 手でスクロールして最新から離れた | 自動スクロールが止まる |
| 最新から離れたまま 30 秒、流れの上で操作 (スクロール・クリック・マウスの移動) が無い | 最新に戻り、自動スクロールを再開する (戻し忘れても配信中に止まったままにならない) |
| 手で最新までスクロールした | 自動スクロールを再開する |
| 最新から離れている | 流れの右下に「最新へ」のボタンを出す。押すと最新に戻り、自動スクロールを再開する |
| 「最新へ」のボタン | 自動で最新に戻るまでの残り時間を、ボタンの中の帯が縮んでいく動きで表す。流れの上で操作すると満ちた状態からやり直す (シフィさん、2026-10-09)。動きを減らす設定 (`prefers-reduced-motion`) では、帯を動かさず残りの秒数を文字で出す |
| 自動スクロールを止める設定 | 無い (止まったままにならないことを優先する。今の「自動スクロール」のチェックと「最新に戻る」のボタンは外す) |

**状態 (異常のときだけ目立つ):**

状態の行 (下端の 1 行) に、左から 点・コメント数・直近 1 分のコメント数・コメントした人数、右端に主の配信のリアクションの累計 (絵文字は色を抜く) を控えめに出す。

| 状況 | 結果 |
|------|------|
| 接続中 | 状態の行の左端の小さい点 (success の色) だけ |
| 接続が切れた・取得の失敗が続いている | 状態の行を error の面にし、理由を文字で出す |
| 未接続 | 流れの場所に接続の入力を出す (配信の準備なので操作してよい) |

## 制約・不変条件（Boundaries）

| 制約 | 理由 |
|------|------|
| 最大同時接続数は32 | メモリとAPI負荷の上限。超過時は接続を拒否する |
| メッセージ重複排除キーは `connection_id:message_id` の複合キー | connection_idなしだと異なる配信間でYouTubeのmessage_idが衝突する可能性がある |
| バックエンドは接続ごとに受け取った message_id を覚え、再び届いたものを捨てる | YouTube は同じ接続に同じメッセージを再送することがある（実測: 同じ id・同じ timestampUsec が約60秒で4回）。フロントの重複排除だけでは、TTS・WebSocket・視聴者の集計に重複が流れる |
| ポーリング間隔は1,500ms | YouTube APIのレート制限に準拠。短くするとAPIブロックのリスクがある |
| 初見さん判定は `save_message`（`upsert_viewer_stream`含む）の**後**に実行する | 判定前にDBにデータが存在している必要がある |
| システムメッセージ（`message_type == "system"`）は初見判定の対象外 | システムメッセージはユーザーの発言ではない |
| 配信内コメント数カウンタはメモリ上のHashMapで管理し、DBには保存しない | O(1)のパフォーマンスが必要。再接続時はDBから復元可能 |
| ギフトの handle から channel_id を引くとき、候補が2件以上なら特定しない | 別人に紐付けると読み仮名・初見判定・集計がすべて誤る。特定しないだけなら表示と保存は正しい |
| 特定できないギフトの channel_id は `""` とし、viewer_profiles を作らない | 仮のIDでプロフィールを作ると本物の channel_id と二重になる |
| ジュエル数は本文から読めたときだけ記録し、推測しない | ギフトの価格は YouTube 側で変わりうる |
| リアクションは更新（`emojiFountainDataEntity`）単位で記録し、1秒ごとの時刻を推測で振らない | バケットの時間順が応答から判別できない。推測した時刻は分析結果を誤らせる |
| リアクションの件数は `reactionsData[].reactionCount` だけから数える | `totalReactions` と二重の出どころを持つと、食い違ったときにどちらが正か決められない |

## 未決事項（Open Questions）

「配信中の画面」(2026-10 の作り直し) で、まだ決めていないこと。決まったら「振る舞い」に移す。

| 項目 | 仮の扱い |
|------|------|
| 複数の配信をまとめてつなぐ手段 | 未設計。コラボで続けて接続するときの手間を減らす案 (シフィさん、2026-10-09)。今は「つないだら操作パネルを隠す」を無効にすれば続けて接続できる |
| スパチャのティアの色の読みやすさ | YouTube の色のまま出す (シフィさん、2026-10-09)。オレンジのティアは白い文字で WCAG AA (4.5:1) に届かないが、承知の上で YouTube に合わせる。設定で読みやすさを調整するオプションは未設計 |
| WebSocket サーバーの状態の置き場所 | 今は上のヘッダーにある。操作パネルの中に移す案。異常 (起動に失敗) は状態の行 |
| 主の配信をどれにするか | 最初の接続 (本人の配信とは限らない)。ログイン中のチャンネルと一致する接続を主にする案もあるが、ログインしていない・テストで他の人の配信に接続する場合があるので、今は採らない。視聴者情報 (読み仮名・メモ) の配信者も最初の接続に固定されている。コラボで相手側の視聴者を扱うときの整理が要る |
| 「流れたもの」の欄の 30 秒・1/8・2 件などの値 | 試作での仮の値。配信で使って見直す |
| コメントのピン留め、視聴者での絞り込み | 未設計 (ブリーフ §4) |
| 参加型の一覧 (参加中・待機) と、パネルの配置の選択 | 未設計。参加の受け付け方も未決 (ブリーフ「機能への影響」) |
| 同時接続数・いいね数 | 取得していない。状態の行には出せるものだけを出す |
| 配信中に使わない操作を隠す設定 | 未設計 (ブリーフ §4) |

## バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `connect_to_stream` | `url: String, chat_mode: Option<String>` | `ConnectionResult` | 配信に接続（connection_idを返す） |
| `disconnect_stream` | `connection_id: u64` | `()` | 特定の接続を切断 |
| `disconnect_all_streams` | なし | `()` | 全接続を一括切断 |
| `get_connections` | なし | `Vec<ConnectionInfo>` | アクティブな全接続情報を取得 |
| `set_chat_mode` | `connection_id: u64, mode: String` | `Result<bool, Error>` | チャットモード切り替え（watchチャネル経由で次回ポーリング時に適用） |
| `get_connection_reactions` | `connection_id: u64` | `Result<ReactionSummary, Error>` | その接続の配信（video_id）のリアクション累計と、直近60秒の更新（F5リロード後のメーター復元用） |

## データモデル

### GuiChatMessage

フロントエンドに送信されるチャットメッセージ。

```rust
pub struct GuiChatMessage {
    pub id: String,                           // メッセージID（YouTube上で一意）
    pub timestamp: String,                    // タイムスタンプ（RFC3339/ISO8601形式、UTC）
    pub timestamp_usec: String,               // マイクロ秒単位タイムスタンプ
    pub author: String,                       // チャットユーザー名
    pub author_icon_url: Option<String>,      // プロフィール画像URL
    pub channel_id: String,                   // ユーザーのチャンネルID
    pub content: String,                      // メッセージテキスト（絵文字は代替テキスト）
    pub runs: Vec<MessageRun>,                // テキストと絵文字の混在リスト
    pub message_type: String,                 // メッセージ種別
    pub amount: Option<String>,               // スーパーチャット/スーパーステッカーの金額
    pub is_member: bool,                      // メンバーシップ登録済み
    pub is_first_time_viewer: bool,           // 初見さん（配信者チャンネルでの初コメント）
    pub in_stream_comment_count: Option<u32>, // この配信（video_id単位）でのコメント回数
    pub metadata: Option<GuiMessageMetadata>, // メタデータ
    pub connection_id: u64,                   // この接続のID（多接続識別用）
    pub platform: String,                     // 配信プラットフォーム（"youtube"等）
    pub broadcaster_name: String,             // 配信者名
    pub author_reading: Option<String>,       // 届いたときの読み仮名（視聴者情報に登録済みなら。読み上げと同じ値）
}
```

### MessageRun

テキストと絵文字の混在表現。

```rust
#[serde(tag = "type")]
pub enum MessageRun {
    Text { content: String },
    Emoji { emoji_id: String, image_url: String, alt_text: String }
}
```

**例:** "こんにちは 😀 元気？" → `[Text("こんにちは "), Emoji(...), Text(" 元気？")]`

### MessageType

| 種別 | 説明 | 付加情報（metadata内） |
|------|------|----------------------|
| `text` | 通常のチャットメッセージ | なし |
| `superchat` | スーパーチャット | `amount`（金額文字列）、`superchat_colors` |
| `supersticker` | スーパーステッカー | `amount`（金額文字列）、`superchat_colors` |
| `membership` | メンバーシップ新規/更新 | `milestone_months`（マイルストーン月数、新規はNone） |
| `membership_gift` | メンバーシップギフト配布 | `gift_count`（ギフト数） |
| `gift` | ジュエルで送るギフト。`content` はギフト名 | `gift_name`、`gift_image_url`、`jewel_count`（取れなければ None） |
| `system` | システムメッセージ | なし |

### GuiMessageMetadata

```rust
pub struct GuiMessageMetadata {
    pub amount: Option<String>,                // 金額
    pub milestone_months: Option<u32>,         // メンバーシップマイルストーン月数
    pub gift_count: Option<u32>,               // メンバーシップギフト数
    pub gift_name: Option<String>,             // ギフト名（gift のみ）
    pub gift_image_url: Option<String>,        // ギフト画像URL（gift のみ）
    pub jewel_count: Option<u32>,              // ジュエル数（gift で取れたときのみ）
    pub badges: Vec<String>,                   // バッジ識別子
    pub badge_info: Vec<BadgeInfo>,            // バッジ詳細
    pub is_moderator: bool,                    // モデレータ
    pub is_verified: bool,                     // 検証済みアカウント
    pub superchat_colors: Option<SuperChatColors>,
}

pub struct SuperChatColors {
    pub header_background: String,  // "#RRGGBB"
    pub header_text: String,
    pub body_background: String,
    pub body_text: String,
}

pub struct BadgeInfo {
    pub badge_type: String,        // "member", "moderator", "verified", etc.
    pub label: String,             // 表示ラベル
    pub tooltip: Option<String>,   // ツールチップテキスト
    pub image_url: Option<String>, // バッジ画像URL
}
```

### ReactionUpdate / GuiReactionUpdate / ReactionSummary

```rust
/// 1回のリアクション更新（コア。WebSocket と DB 保存に使う）
pub struct ReactionUpdate {
    pub update_time_usec: i64,                 // YouTube 側の更新時刻（マイクロ秒）
    pub duration_seconds: u32,                 // この更新が表す秒数（バケット数）
    pub counts: BTreeMap<String, u32>,         // 絵文字 → 件数（0件の絵文字は含めない）
    pub total: u32,                            // counts の合計
}

/// Tauri イベント `chat:reaction` のペイロード
pub struct GuiReactionUpdate {
    pub connection_id: u64,
    #[serde(flatten)]
    pub update: ReactionUpdate,
}

/// `get_connection_reactions` の戻り値
pub struct ReactionSummary {
    pub totals: BTreeMap<String, u64>,         // 配信（video_id）単位の絵文字別累計
    pub recent: Vec<ReactionUpdate>,           // update_time_usec が現在から60秒以内の更新（古い順）
}
```

### ChatMode

| モード | 説明 |
|-------|------|
| `TopChat` | 重要なメッセージのみ（YouTubeアルゴリズムで選別） |
| `AllChat` | すべてのメッセージ（時系列順） |

### ConnectionResult

```rust
pub struct ConnectionResult {
    pub success: bool,
    pub stream_title: Option<String>,
    pub broadcaster_channel_id: Option<String>,
    pub broadcaster_name: Option<String>,
    pub is_replay: bool,                       // アーカイブ再生判定
    pub error: Option<String>,
    pub session_id: Option<String>,            // DBセッションID
    pub connection_id: u64,                    // 接続ID（多接続識別用）
}
```

### ConnectionInfo

フロントエンドに公開する接続情報。

```rust
pub struct ConnectionInfo {
    pub id: u64,
    pub platform: Platform,                    // "youtube"
    pub stream_url: String,
    pub stream_title: String,
    pub broadcaster_name: String,
    pub broadcaster_channel_id: String,
    pub is_monitoring: bool,                   // 現在は常にtrue（接続中=監視中）
    pub is_cancelling: bool,
}
```

## InnerTube API

### エンドポイント

| 用途 | URL |
|------|-----|
| メッセージ取得 | `https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={api_key}` |
| アカウントメニュー（認証検証） | `https://www.youtube.com/youtubei/v1/account/account_menu` |

### リクエスト形式

```json
{
    "context": {
        "client": {
            "clientName": "WEB",
            "clientVersion": "{extracted_version}"
        }
    },
    "continuation": "{continuation_token}"
}
```

### Continuation Token

メッセージ取得の継続に使用するトークン。Protocol Buffer形式でエンコードされている。

| データタイプ | 優先度 | 用途 |
|-------------|-------|------|
| `invalidationContinuationData` | 1 | 新メッセージ検出時に即座に通知 |
| `timedContinuationData` | 2 | 指定時間後にポーリング |
| `reloadContinuationData` | 3 | フォールバック |

### チャットモード切り替え

#### コマンドレベルの動作

`set_chat_mode` コマンドは `tokio::sync::watch` チャネル経由で監視タスクにモード変更要求を送信する。
InnerTubeClient の所有権は監視タスクに一本化されており、コマンドハンドラからは直接アクセスしない。

```
set_chat_mode(connection_id, mode)
  → StreamConnection.chat_mode_tx.send(mode)  // watch チャネルで送信
  → 監視ループがポーリングのたびに chat_mode_rx の最新値（選ばれたモード）を読む
  → クライアントの現在のモードと違えば client.set_chat_mode(mode) を適用（失敗したら次のポーリングでやり直す）
  → 次のAPIリクエストで新しいcontinuation tokenが使用される
```

> **注意**: モード変更は非同期で適用される。`set_chat_mode` コマンドの戻り値 `true` は「要求受付成功」を意味し、「即時適用完了」ではない。

#### 切り替えの振る舞い

| 状況 | 結果 |
|------|------|
| 「全て」を選んでアプリを再起動する | 前回選んだ「全て」のまま始まる（config.toml の `chat_display.chat_mode`、[09_config.md](09_config.md)） |
| F5 リロード | config.toml の値でボタンの表示を戻す（選択はすべての接続に送るので、接続中のモードと一致する） |
| 接続中に「全て」を選ぶ | 次のポーリングから「全て」で取得する |
| continuation token の書き換えに失敗した（形式が想定外など） | warn ログを出し、次のポーリングで新しい token に対してやり直す。反映されるまで毎回試す |
| 反映前に別のモードを選び直した | 最後に選んだモードだけを反映する |

#### InnerTubeClient内部の切り替え方式

continuation token の中のチャットモードの値を書き換える（Binary Modification）。

**token の形式（実データで確認）:**

| 項目 | 内容 |
|------|------|
| 文字 | URL-safe base64（`-` `_` を使う） |
| パディング | 付くことがあり、`=` はパーセントエンコードされて `%3D` で届く（観測では約26%） |
| 構造 | protobuf。field 119693434（メッセージ）→ field 16（メッセージ）→ field 1（varint）がチャットモード。4 = TopChat、1 = AllChat |

**書き換え:**

1. `%3D` を `=` に戻し、パディングの有無どちらでも URL-safe base64 として読む
2. protobuf として上の経路をたどり、field 1 の値を書き換える（1 と 4 はどちらも1バイトなので長さは変わらない）。それ以外のバイトは変えない
3. URL-safe base64（パディング無し）で書き戻す

| 入力 | 結果 |
|------|------|
| field 16 の field 1 = 4 の token を AllChat に | field 1 だけが 1 になった token |
| 末尾が `%3D` の token | 同じく書き換えられる |
| 経路のどこかが無い・protobuf として読めない | 失敗（None）。token は変えない |
| すでに目的のモード | そのまま成功 |

経路はバイト列の見た目ではなく protobuf の構造でたどる。見た目で探すと、時刻などの別の値の中にある同じバイト列を書き換えてしまう（実データで8件観測）。

### 認証ヘッダー（メンバー限定配信用）

```
Authorization: SAPISIDHASH {hash}
Cookie: SID=...; HSID=...; SSID=...; APISID=...; SAPISID=...
X-Origin: https://www.youtube.com
Origin: https://www.youtube.com
```

## 接続処理フロー

```
1. ユーザーがURLを入力 → connect_to_stream呼び出し
        ↓
2. URLからビデオIDを抽出
        ↓
3. YouTubeページをフェッチ、以下を抽出:
   - API Key
   - Client Version
   - Continuation Token
   - 配信者情報（チャンネルID、名前）
   - チャットモード切替用トークン
        ↓
4. InnerTubeクライアントを初期化
        ↓
5. 新規セッションをDBに作成
        ↓
6. 配信内コメント数カウンタを初期化
   ├─ 同一video_idのセッションがDBに存在する場合 → DBから集計して復元
   └─ 新規の場合 → 空のHashMapで初期化
        ↓
7. チャット監視タスクを起動（tokio::spawn）
        ↓
8. ConnectionResultをフロントエンドに返却
```

### メッセージ復元（未実装）

同一配信への再接続時のメッセージ復元は現在未実装。各接続は新規セッションとして開始される。
配信内コメント数カウンタ（`in_stream_comment_counts`）のみDBから復元される。

## チャット監視タスク

```
┌─ ループ（1.5秒ごと）─────────────────────────┐
│ 1. CancellationTokenでキャンセル確認          │
│ 2. fetch_chat()でAPI呼び出し                  │
│    ├─ 応答待ちの間も切断を受け付ける           │
│    ├─ 成功 → 連続失敗回数を0に戻す            │
│    ├─ 失敗 → 連続失敗回数を1増やす            │
│    └─ 新しいcontinuation tokenを取得          │
│ 3. chat_mode_rx でモード変更要求を確認         │
│    └─ 変更あり → client.set_chat_mode(mode)  │
│ 4. この接続で受け取り済みの id を捨てる        │
│ 5. 残りを 1 トランザクションで処理（1件ずつ）: │
│    ├─ ギフトなら handle から channel_id を特定  │
│    ├─ 配信内コメント数カウンタ更新              │
│    ├─ DBに保存（save_message）                │
│    │   ├─ INSERT OR IGNORE (messages)         │
│    │   ├─ upsert_viewer_profile               │
│    │   └─ upsert_viewer_stream(video_id)      │
│    └─ is_first_time_viewer(video_id)で初見判定 │
│ 6. 各メッセージを送る:                         │
│    ├─ GuiChatMessageに初見・回数を付与し、     │
│    │   Tauriイベントを発行                    │
│    ├─ WebSocket に配信                        │
│    └─ TTS キューに追加                        │
│ 7. リアクション更新を処理:                     │
│    ├─ 前回以下の update_time_usec なら捨てる   │
│    ├─ DBに保存（reactions）                   │
│    ├─ Tauriイベント chat:reaction を発行       │
│    └─ WebSocket に Reaction を配信            │
│ 8. 次の取得まで待つ（通常1500ms。失敗が続くと │
│    延ばし、15回続いたら切断して抜ける）        │
└───────────────────────────────────────────────┘
```

### 初見さん判定

配信者チャンネルにおいて初めてコメントした視聴者を判定する。

#### データ構造: viewer_streams テーブル

各視聴者がどの配信にコメントしたかを記録する。

```sql
CREATE TABLE viewer_streams (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    viewer_profile_id INTEGER NOT NULL,     -- viewer_profiles.id
    video_id TEXT NOT NULL,                 -- 配信のvideo_id
    first_comment_at TEXT NOT NULL,         -- この配信での最初のコメント時刻
    last_comment_at TEXT NOT NULL,          -- この配信での最新のコメント時刻
    message_count INTEGER DEFAULT 1,        -- この配信でのコメント数
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (viewer_profile_id) REFERENCES viewer_profiles(id) ON DELETE CASCADE,
    UNIQUE(viewer_profile_id, video_id)
);
```

#### 判定方法

`viewer_streams` テーブルで、視聴者の最古の `video_id` が現在の配信の `video_id` と一致するかで判定する。

```
メッセージ受信時:
  1. save_message → upsert_viewer_profile (viewer_profile_id を取得)
                  → upsert_viewer_stream(viewer_profile_id, video_id)
  2. is_first_time_viewer(broadcaster_id, channel_id, video_id) で判定:
     oldest_video_id = SELECT video_id FROM viewer_streams vs
                       JOIN viewer_profiles vp ON vs.viewer_profile_id = vp.id
                       WHERE vp.broadcaster_channel_id = ? AND vp.channel_id = ?
                       ORDER BY vs.first_comment_at ASC LIMIT 1
     msg.is_first_time_viewer = (oldest_video_id == current_video_id)
```

#### 注意事項

| ポイント | 説明 |
|---------|------|
| 配信者スコープ | `viewer_profiles.broadcaster_channel_id` 経由で配信者スコープを維持 |
| 永続性 | `viewer_streams` テーブルに基づくためアプリ再起動後も正確 |
| 配信内一貫性 | DB問い合わせのため、再接続後も自動的に正しい判定を維持（メモリキャッシュ不要） |
| 判定タイミング | `save_message`（`upsert_viewer_stream` 含む）の**後**に判定する |
| システムメッセージ | `message_type == "system"` のメッセージは判定対象外 |
| 特定できないギフト | `channel_id == ""` のギフトは判定対象外（false） |
| 将来の拡張 | 視聴者の訪問配信一覧、前回来た配信の特定等に利用可能 |

### 配信内コメント数カウンタ

配信（video_id）単位での視聴者ごとのコメント回数を管理する、メモリ上のカウンタ。

#### データ構造

```rust
/// video_id単位の視聴者コメント数カウンタ
/// key: channel_id, value: コメント回数
type InStreamCommentCounter = HashMap<String, u32>;
```

#### ライフサイクル

| イベント | 動作 |
|---------|------|
| 新規接続 | 空のHashMapを作成 |
| 再接続（同一video_id） | DBから該当セッションのメッセージを集計し、channel_idごとのカウントで初期化。URL の書き方（`watch?v=`・`/live/`・`youtu.be/`）が前回と違っても同じ配信として数える |
| メッセージ受信 | カウンタをインクリメントし、現在値を `in_stream_comment_count` に設定 |
| 切断 | カウンタを破棄（DBにメッセージが保存されているため復元可能） |

#### 再接続時の復元SQL

```sql
SELECT m.channel_id, COUNT(*) as count
FROM messages m
JOIN sessions s ON m.session_id = s.id
WHERE s.video_id = {video_id}
  AND m.message_type != 'system'
  AND m.channel_id <> ''
GROUP BY m.channel_id
```

#### パフォーマンス

| 操作 | 計算量 |
|------|--------|
| カウント参照 | O(1) - HashMap lookup |
| カウント更新 | O(1) - HashMap insert/update |
| 初期化（再接続時） | O(N) - Nはユニーク視聴者数 |

### 受信済み message_id の記憶

接続ごとに、受け取った message_id を直近 10,000 件まで覚える。超えたら古いものから忘れる。
YouTube の再送は数十秒〜数分のうちに届くので、直近だけ覚えれば足りる。件数に上限を設けて、長時間の配信でもメモリを一定に保つ（接続あたり約1MB）。

| 状況（上限を3件とした例） | 結果 |
|------|------|
| A, B, A の順に届く | A, B を処理し、2回目の A を捨てる |
| A, B, C, D, A の順に届く | A, B, C, D を処理する。D を覚えた時点で A を忘れているので、最後の A も処理する |
| message_id が空のメッセージ | 重複の判定をせず、常に処理する（空の id 同士を同じものとみなさない） |
| 切断 | 記憶を破棄する。再接続は新しい接続なので、YouTube が返す直近のコメントも新着として扱う |

### 取得に失敗したとき

YouTube 側の一時的な不調ならそのまま続け、復旧しそうにないときは自動で切断して知らせる。
接続しているつもりで何も取れていない状態を放置しない。

失敗として数えるもの: リクエストのタイムアウト（全体15秒・接続5秒）、ネットワークエラー、成功（2xx）以外の HTTP ステータス、JSON として読めない応答。
応答が返ってきてメッセージが 0 件なのは失敗ではない。

| 連続失敗回数 | 次の取得までの待ち |
|------|------|
| 0（成功） | 1.5秒 |
| 1〜10 | 1.5秒（成功と同じ） |
| 11 | 3秒 |
| 12 | 6秒 |
| 13 | 12秒 |
| 14 | 24秒 |
| 15 | 待たずに切断する |

- 1 回でも成功したら連続失敗回数は 0 に戻る（例: 失敗12回 → 成功 → 失敗 なら、次の待ちは 1.5秒）
- 成功している間の間隔は 1.5秒のまま（制約「ポーリング間隔は1,500ms」）。延ばすのは失敗が続いたときだけ
- 15回目で切断するとき: セッションを閉じ、接続を一覧から消し、エラー「チャットの取得に15回続けて失敗したため切断しました」を表示する（「接続状態遷移」の監視タスク終了と同じ経路）
- 1〜10回目の待ちを今までと同じにするのは、短い不調（数秒〜15秒）で取りこぼす時間を増やさないため

### 設定値

| 項目 | 値 |
|-----|-----|
| ポーリング間隔 | 1,500ms |
| HTTP タイムアウト | 全体15秒、接続5秒 |
| 自動切断するまでの連続失敗回数 | 15回（11回目から待ちを倍々に延ばす） |
| 受信済み message_id の記憶件数（接続ごと） | 10,000件 |
| デフォルトAPI Key | `AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8` |

## Tauriイベント

| イベント名 | ペイロード | 説明 |
|-----------|-----------|------|
| `chat:connection` | `ConnectionResult` | 接続状態変更 |
| `chat:message` | `GuiChatMessage` | 新着メッセージ |
| `chat:reaction` | `GuiReactionUpdate` | リアクション更新（0件の更新・再送は発行しない） |

## フロントエンド

### コンポーネント

| コンポーネント | 役割 |
|--------------|------|
| `ChatTab.svelte` | チャットタブ全体のレイアウト管理（リサイズハンドル含む） |
| `InputSection.svelte` | URL入力、接続開始、「接続設定」ラベル（常時表示、多接続対応） |
| `ConnectionList.svelte` | アクティブ接続アイテムリスト、個別切断ボタン、ジュエル数を取れない接続の注記 |
| `ReactionMeter.svelte` | 接続ごとのライブリアクションの累計と勢い（リアクションのある接続だけ1行ずつ） |
| `ChatDisplay.svelte` | メッセージ一覧表示（配信元色ライン付き） |
| `FilterPanel.svelte` | メッセージフィルタリング、表示制御、メッセージクリア |

### レイアウト構成

> 作り直し中 (2026-10): 振る舞いは「配信中の画面」が正。この節は段階 5.2a〜5.2c の実装と一緒に書き換える。

チャットタブは以下の構成で、接続リスト部分のみドラッグリサイズ可能。

```
┌─────────────────────────────────────────┐
│ 接続設定 [URL入力] [開始] [モード] [統計] │ ← InputSection（固定）
├─────────────────────────────────────────┤
│ 接続中: N                        全切断  │ ← ChatTab内ヘッダー（固定）
├─────────────────────────────────────────┤
│ ┌─ 接続アイテム ──────────────────────┐ │
│ │ ■ 配信者名A                      × │ │ ← ConnectionList（スクロール可能）
│ │   配信タイトルA                     │ │    max-height: ドラッグで制御
│ │ ■ 配信者名B                      × │ │    min-height: アイテム1件分（動的測定）
│ └────────────────────────────────────┘ │
│ ═══════════ リサイズハンドル ═══════════ │ ← Pointer Capture APIで実装
├─────────────────────────────────────────┤
│ [フィルター] [自動スクロール] [設定...]   │ ← FilterPanel
├─────────────────────────────────────────┤
│ ■ 配信者名A ❤ 430 🎉 418 … 勢い 38/分   │ ← ReactionMeter（リアクションのある接続だけ）
├─────────────────────────────────────────┤
│                                         │
│           チャットメッセージ              │ ← ChatDisplay（flex-1）
│           （仮想スクロール）              │
│                                         │
└─────────────────────────────────────────┘
```

| 項目 | 仕様 |
|-----|------|
| リサイズ最小高 | 接続アイテム1件のDOM実測高さ（フォールバック: 50px） |
| リサイズ最大高 | ビューポートの40% |
| デフォルト高 | 120px |
| 接続0件時 | 接続リスト・ヘッダー・リサイズハンドルは非表示 |

### Store状態（chat.svelte.ts）

#### 接続・メッセージ関連

| 状態 | 型 | 説明 |
|-----|-----|------|
| `messages` | `ChatMessage[]` | 全接続のメッセージを統合した配列 |
| `filteredMessages` | `ChatMessage[]` | フィルタ済みメッセージ（derived） |
| `displayedMessages` | `ChatMessage[]` | displayLimit適用後の表示用メッセージ（derived） |
| `connections` | `Map<number, FrontendConnectionState>` | アクティブ接続マップ（connection_id → 状態） |
| `connections[id].jewelCountUnavailable` | `boolean` | その接続で jewel_count = null のギフトを受けたら true（フロントエンドのみの状態。バックエンドには持たない） |
| `connections[id].reactions` | `{ totals: Record<string, number>; recent: ReactionUpdate[]; peakPerMinute: number }` | リアクションメーターの状態。`recent` は update_time_usec が60秒以内のものだけ残す。勢い（/分）は `recent` の total の合計（derived） |
| `isConnected` | `boolean` | いずれかの接続がアクティブ（derived: connections.size > 0） |
| `isConnecting` | `boolean` | 接続処理中の接続が存在（derived） |
| `broadcasterChannelId` | `string \| null` | 最初の接続の配信者チャンネル ID（視聴者タブの既定の配信者。視聴者情報パネルには使わない） |
| `broadcasterChannelIdOf(message)` | `string \| null` | そのメッセージが届いた接続の配信者チャンネル ID。接続が一覧に無い（切断済み）か配信者が分からなければ `null`（視聴者情報パネルの配信者） |
| `chatMode` | `ChatMode` | TopChat / AllChat。起動時・F5 後は config.toml の `chat_display.chat_mode` で初期化し、選び直すたびに保存する |
| `error` | `string \| null` | エラーメッセージ |

#### 表示設定関連

| 状態 | 型 | 説明 |
|-----|-----|------|
| `messageFontSize` | `number` | フォントサイズ（10〜24px、デフォルト13px） |
| `showTimestamps` | `boolean` | タイムスタンプ表示 |
| `autoScroll` | `boolean` | 自動スクロール有効 |
| `displayLimit` | `number \| null` | 表示件数制限（null=無制限） |
| `scrollToLatestTrigger` | `number` | スクロールトリガー（インクリメントで発火） |

#### 視聴者情報パネル関連（ChatDisplayコンポーネントのローカル状態）

| 状態 | 型 | 説明 |
|-----|-----|------|
| `selectedViewer` | `{ channelId, displayName, iconUrl?, message, broadcasterChannelId } \| null` | 選択中の視聴者（パネル表示は `selectedViewer !== null` で制御）。`broadcasterChannelId` は一覧でコメントをクリックしたときに `broadcasterChannelIdOf(message)` で決め、パネル内のコメントを押しても変えない |

※ 視聴者カスタム情報はAPI経由でオンデマンド取得（キャッシュなし）。

### フィルタ機能

```typescript
interface ChatFilter {
    showText: boolean;        // 通常チャット表示
    showSuperchat: boolean;   // スーパーチャット/ステッカー/ギフト表示
    showMembership: boolean;  // メンバーシップ関連表示
    searchQuery: string;      // 検索クエリ（著者/コンテンツ）
}
```

### ユーザー操作

| 操作 | 動作 |
|-----|------|
| URL入力 + 「開始」クリック | `connect_to_stream`呼び出し（複数回実行可能） |
| 個別「切断」クリック | `disconnect_stream(connection_id)`呼び出し |
| 「全切断」クリック | `disconnect_all_streams`呼び出し |
| 「クリア」クリック | メッセージ配列をクリア（確認ダイアログ付き） |
| チャットモード切り替え | `set_chat_mode` で全接続に変更要求を送信（次回ポーリングで適用）し、config.toml に保存する |
| フィルタ選択 | ローカルでフィルタリング |
| フォントサイズ変更 | ローカル設定を更新 |
| F5リロード | バックエンドの接続状態を `get_connections` で取得し、フロントエンドに復元 |

### 接続設定の統計（InputSection）

> 作り直し中 (2026-10): 振る舞いは「配信中の画面」が正。この節は段階 5.2a〜5.2c の実装と一緒に書き換える。

URL 入力欄の右に、画面のメッセージ一覧の統計を出す。未接続のときはすべて 0（薄く表示）。

| 表示 | 値 |
|-----|-----|
| 件 | 画面のメッセージ一覧の件数（「クリア」で 0） |
| /分 | 直近 60 秒に投稿されたコメントの数（投稿時刻 `timestamp_usec` で数える）。コメントが止まれば下がっていく（5 秒ごとに数え直す） |
| 人 | 画面のメッセージ一覧にいる視聴者の数（`channel_id` ごと。視聴者を特定できないギフトは数えない） |

| 状況（/分、いま 12:00:00） | 結果 |
|------|------|
| 11:58:00・11:59:10・11:59:50 に投稿されたコメント | 2 |
| 最後のコメントが 11:59:50 で、そのあと 60 秒コメントが無い | 12:01:00 までに 0 になる |
| アーカイブ（投稿時刻が過去）のコメントだけ | 0 |

| 状況（人） | 結果 |
|------|------|
| UCa が 3 件、UCb が 1 件、視聴者を特定できないギフトが 1 件 | 2 |

件・人はストアが受信のたびに差分で数える（全件を数え直さない）。/分は一覧の末尾から辿り、投稿時刻が 90 秒より前のメッセージに当たったら止める
（一覧は到着順。複数の接続で到着が前後しても数十秒に収まる前提）。

### フロントエンド状態の復元（F5リロード後）

ページリロード時にバックエンドのアクティブ接続をフロントエンドに復元する。

| 項目 | 詳細 |
|------|------|
| メソッド | `chatStore.restoreConnections()` |
| 呼び出しタイミング | `+page.svelte` の `onMount` で `setupEventListeners()` の後に実行 |
| 実行内容 | `get_connections` コマンドで全接続情報を取得し、`FrontendConnectionState` に変換してMapに追加。各接続の `get_connection_reactions` でリアクションメーターを復元する（失敗したらその接続のメーターは空から始める） |
| 復元後の動作 | バックエンドの監視ループは継続しているため、復元後は新着メッセージが正常に受信される |
| 失敗時 | `console.warn` でログ出力、アプリは正常に動作（接続リストが空のまま） |

### 接続状態遷移（個別接続単位）

```
  開始                                    切断
(なし) → Connecting → Connected → Disconnecting → (削除)
                           │
                           ├─ (取得に15回続けて失敗) → (削除 + エラー通知)
                           └─ (監視タスク異常終了) → (削除 + エラー通知)

Connecting → (接続失敗) → (削除 + エラー通知)
```

多接続モデルでは各接続が独立した状態を持つ。グローバルな `paused` 状態は存在しない。

| 状態 | 説明 | UI表示 |
|-----|------|-------|
| `connecting` | 接続処理中（API応答待ち） | 開始ボタン無効化、「接続中...」表示。その接続の切断ボタンは無効 |
| `connected` | 接続済み、メッセージ受信中 | 接続リストにエントリ表示 |
| `disconnecting` | 切断処理中 | 切断アニメーション |

### 同時接続

| 項目 | 仕様 |
|-----|------|
| 最大同時接続数 | 32 |
| メッセージ統合 | 全接続のメッセージをグローバルバッファに統合 |
| 重複排除 | 複合キー `connection_id:message_id` で O(1) チェック |
| 配信元識別 | `broadcaster_channel_id` のハッシュ値から色を決定 |
| メッセージ表示 | 主の配信 (最初の接続) 以外のコメントに配信者名の札を付ける。強さ (優先順位) は変えない。札は省略しない (「メッセージ表示仕様」) |

## メッセージ表示仕様

振る舞いは「配信中の画面」(1 行の形・優先順位と強調)。ここは実装の詳細。
コンポーネントは `ChatMessage.svelte`、見た目の値は [docs/design/tokens.md](../design/tokens.md) のトークンだけを使う (`design-lint.config.js` の `DESIGN_MIGRATED`)。
試作は `src/routes/design-proto/Row.svelte`。

### 行の根

| 項目 | 値 |
|------|------|
| 要素 | `div[role=button][tabindex=0]`。クリック・Enter で視聴者情報パネルを開く |
| 属性 | `data-message-id`、`data-priority` (1〜5、`messagePriority` の結果)、`data-origin` (`primary` / `partner`、`chatStore.originOf`) |
| 文字の大きさ | 行の `font-size` = 表示の設定 (`messageFontSize`、10〜24px)。中の文字は em の段階 (`text-chat-body` 1em / `text-chat-meta` 0.85em / `text-chat-amount` 1.15em) |
| 文字色 | `text-fg-1` (周りの色を引き継がない) |
| 選択中 (パネルの過去コメントから飛んだ先、3 秒) | `outline-2 -outline-offset-2 outline-accent` |

### 優先順位 2〜5 の行

```
[アイコン] [配信元の札] [強調の印] 名前 (読み仮名) [バッジ] [金額の札] 本文… #N 時刻
```

- 1 つの段落 (`<p>`) に続けて書き、幅が足りなければ折り返す。`truncate`・`text-overflow` は使わない
- 余白 `px-3 py-1.5`。マウスを載せると `fg-1` を 8% 重ねる

| 要素 | 表し方 |
|------|------|
| アイコン | `size-chat-icon` (1.5em) の丸。画像が無い・読めなければ頭文字 |
| 配信元の札 (`data-testid="source-label"`) | 相手の配信のときだけ。`bg-surface-highest`・`text-fg-2`・`text-chat-meta` に配信者名 |
| 強調の印 | 2: `bg-highlight` に `text-surface` の「初見」+ 行の背景 `bg-highlight/20` / 3: `border-highlight` の「初見」 / 4: `text-highlight` の「●」(`aria-label="配信内で初めてのコメント"`) / 5: なし |
| 名前 (`data-testid="author-name"`) | `font-bold`。メンバーは `text-member-text`。ただし優先順位 2 の行では通常の色 |
| 読み仮名 | `chatStore.readingOf(message)` (パネルで保存した値) があればそれ、無ければ `author_reading`。空なら出さない。`text-chat-meta`・`text-fg-2` で `(たなか)` |
| バッジ | `badge_info` の画像 (`size-chat-badge`、読めなければ消す)。メンバーで画像が無ければ `border-member-text` の札「メンバー」。モデレーター 🔧・認証済み ✓ |
| 金額の札 (`data-testid="paid-label"`) | 相手の配信のスパチャ等のとき。`border-outline` の枠に `paidLabelOf` の文言 |
| 本文 | `runs` (テキストと絵文字)。絵文字は `h-chat-emoji` (1.3em)。ステッカーは本文を出さない (`content` は代わりの文字「[Sticker]」) |
| #N・時刻 (`data-testid="message-time"`) | `text-chat-meta`・`text-fg-3` (優先順位 2 の行では `text-fg-2`)。時刻は表示の設定で消せる (HH:MM:SS、ローカル時刻) |

### 優先順位 1 の面

主の配信のものだけ。面は `mx-2 my-1 rounded-control` (浮いて見える理由がある数少ない要素なので角丸)。

| 種類 | 面 | 見出し | 本文 |
|------|------|------|------|
| スパチャ・ステッカー (色あり) | 背景 `body_background` | 背景 `header_background`・文字 `header_text`: アイコン、名前 (読み仮名)、#N・時刻、右端に金額 (`text-chat-amount`) | 文字 `body_text` (`data-testid="paid-body"`)。本文が無ければ出さない。ステッカーは出さない |
| スパチャ・ステッカー (色なし) | `bg-surface-highest`・`border-outline` | 同上 (トークンの色) | 同上 |
| メンバーシップ・マイルストーン・メンバーシップギフト | `bg-member`・`text-on-member` | アイコン、名前、#N・時刻、右端に `paidLabelOf` (新規メンバー / メンバー N か月 / メンバーシップを N 件ギフト。件数が分からなければ「メンバーシップのギフト」) | メッセージがあれば |
| ギフト (ジュエル) | `bg-surface-highest`・`border-outline` | ギフトの画像 (`size-chat-gift`、読めなければ消す)、「{author} が {gift_name} を送りました」、#N・時刻、右端に「N ジュエル」(`data-testid="gift-jewels"`、jewel_count があるときだけ) | なし |

- 札の文言は `src/lib/design/paid-label.ts` の `paidLabelOf` (流れたもの・お礼の一覧と共通)
- 視聴者を特定できないギフトは #N を出さない (`in_stream_comment_count` が null)
- 件数・か月数・ギフト名は `message_type` から `metadata` に載せる。パーサーが `metadata` を付けない種別 (メンバーシップギフト) でも、空の `metadata` を作って載せる (`GuiChatMessage::from`)

### 詰めた形 (流れたものの欄・お礼の一覧)

`ChatMessage` の `compact`。流れたものの欄の新しい 2 件と、お礼の一覧の行に使う。

| 項目 | 表し方 |
|------|------|
| アイコン | 出さない |
| 優先順位 1 | 面にせず 1 行: 時刻 (`text-fg-3`、表示の設定によらず出す。お礼で順に読むため)・札 (`data-testid="paid-chip"`)・名前 (読み仮名)・本文 |
| 札の色 | スパチャ・ステッカーはティアの見出しの色 (`header_background`・`header_text`)、メンバーシップ系は `bg-member`、色の無いものとギフトは `border-outline` の枠 |
| ギフト (ジュエル) | 札は「N ジュエル」、本文の代わりに「{gift_name} を送りました」。ジュエル数が無ければ札がギフト名で、本文なし |
| 優先順位 2〜5 | 流れと同じ (アイコンだけ出さない) |

### 流れたものの欄とお礼の一覧

振る舞いは「配信中の画面」の「流れたもの」「配信の最後のお礼」。部品は `ChatDisplay.svelte` (組み立て)・`FlowedSection.svelte` (欄)・`ThanksList.svelte` (一覧)。
中身は `src/lib/design/flow.ts` の純粋な関数で決める (試作と共用)。

| 項目 | 実装 |
|------|------|
| 候補 | `chatStore.supporters` = 主の配信の優先順位 1、`chatStore.newcomers` = 優先順位 2 (相手の配信を含む)。届いた順。フィルタ・表示件数によらず、一覧にあるもの (`chatStore.messages`) すべてから取る (お礼の一覧はこのまま)。「クリア」で消える |
| 見えているいちばん上の行 | virtua の `findItemIndex(getScrollOffset())` で、表示中の一覧 (`displayedMessages`) の行を取る。一部だけ見えている行も見えているとみなす。スクロール・一覧の変化・流れの高さの変化のたびに測り直す |
| 流れたもの | `flowedAbove(候補, chatStore.filteredMessages, 見えているいちばん上の行)`: フィルタ後の一覧でその行より前にある候補を、新しいものを先に。何も見えていなければ空 |
| 欄 | `<section aria-label="流れた新規さん">`・`<section aria-label="流れたスパチャ等">`。見出し `h3` に名前と件数。新しい 2 件は詰めた形の行、それより前は短い札 (`data-testid="flowed-chip"`): 優先順位 1 は「(ティアの色または member の点) 札の文言 名前」、新規さんは「初見 名前」。`max-h-1/8`、超えたら欄の中でスクロール |
| 一覧を開くボタン | 「この配信のスパチャ等の一覧 N」。候補があり、一覧を閉じているときだけ |
| 一覧 | `<section aria-label="この配信のスパチャ等">`。見出し `h2` に「N 件・時刻順」と「閉じる」。`max-h-1/2`、行は詰めた形 |
| 行のクリック | 欄の行・一覧の行も、流れと同じく視聴者情報パネルを開く。短い札は押せない |
| 流れの高さが変わった (欄・一覧の開閉、ウィンドウの大きさ) | 自動スクロール中なら最新に合わせ直す (ResizeObserver) |

### システムメッセージ

本文だけを `text-fg-2` で出す (名前・アイコンなし)。

### 初見さん判定と配信内コメント回数

判定の仕様は「振る舞い」の「初見さん判定」と「チャット監視タスク」の「配信内コメント数カウンタ」。表示は上の強調の印と #N。

| 状態 | 優先順位 | 表示 |
|------|------|------|
| 初見 + 配信内 1 回目 | 2 | 薄い背景 + 塗りの「初見」 + `#1` |
| 初見 + 配信内 2 回目以降 | 3 | 枠の「初見」 + `#N` |
| 初見ではない + 配信内 1 回目 | 4 | 「●」 + `#1` |
| 初見ではない + 配信内 2 回目以降 | 5 | `#N` |
| 回数が分からない (視聴者を特定できないギフトなど) | 5 | #N なし |

#### クリック時

メッセージクリックで**視聴者情報パネル**（ViewerInfoPanel）を表示。詳細は「視聴者情報パネル」セクションを参照。
パネルで読み仮名を保存すると `chatStore.setViewerReading` で記録し、画面にある同じ視聴者 (同じ配信者のもと) の行にもすぐ出す。

### スクロール動作

#### 自動スクロール

振る舞いは「配信中の画面」の「自動スクロール」が正。ここは実装の詳細。

| 項目 | 実装 |
|-----|-----|
| 状態 | `src/lib/utils/auto-return.ts` の `AutoReturnState`。「最新を追っている」か「離れていて、期限 (時刻) に戻る」のどちらか。遷移は純粋関数 (`leaveLatest`・`activity`・`reachLatest`・`tick`) で、`ChatDisplay.svelte` が呼ぶ |
| 戻るまでの時間 | `AUTO_RETURN_MS` = 30 秒 |
| 追うとき | 新着で `queueMicrotask` から virtua の `scrollToIndex(最後, { align: 'end' })`。流れの高さが変わったときも追っていれば合わせ直す |
| 手でのスクロールの判定 | 流れの上の `wheel`・`pointerdown`・`keydown`・`touchstart` から 1 秒以内のスクロールだけを手でのスクロールとみなす。virtua が行の高さを測り直すときのスクロールでは止めない (たくさん届いても追うのをやめない)。手で離れたら `scrollBy(0)` で virtua の合わせ直し (`scrollToIndex` は行の高さが測れるまで目標へ合わせ直し続ける) を今の位置に置き換え、最新へ引き戻されないようにする |
| 最新にいるかの判定 | `getScrollOffset() + getViewportSize() >= getScrollSize() - 4px`。最新にいるとき (手でもプログラムでも) は追うのを再開する |
| 流れの上の操作 | `pointermove`・`click` で期限を延ばす (`activity`) |
| 期限の確認 | 離れている間だけ 1 秒ごとに `tick` |
| 視聴者情報のパネルから過去のコメントへ移ったとき | `leaveLatest` (手で離れたのと同じ扱い) |
| 表示するコメントが無くなったとき (クリア・つなぎ直し) | 追う状態に戻す (離れる先が無いため) |

#### 「最新へ」のボタン

`LatestButton.svelte`。離れていて、表示するコメントがあるときに流れの右下に出す。押すと `reachLatest`。

| 項目 | 実装 |
|-----|-----|
| 残り時間のバー | 期限が延びるたびにバーだけを作り直し (`{#key deadline}`)、残りの割合から 0 まで `scaleX` を CSS のアニメーションで縮める。ボタンごと作り直すと、マウスを載せて期限が延びるたびに入れ替わって押せなくなる |
| 動きを減らす設定 | バーを隠し、「あと N 秒で戻ります」を出す (1 秒ごとに更新) |

### 表示設定

#### フォントサイズ

| 項目 | 値 |
|-----|-----|
| 範囲 | 10〜24px |
| デフォルト | 13px |
| 調整 | A-/A+ ボタン（±1px） |
| 永続化 | `config.toml` に保存、次回起動時に復元 |

※設定の詳細は[設定機能仕様](09_config.md)を参照

**影響範囲:**
- メッセージ本文
- 著者名
- 絵文字サイズ（フォント+4px）

※タイムスタンプ（10px）、バッジ（9px）は固定

#### 表示切り替え

| 項目 | デフォルト |
|-----|----------|
| タイムスタンプ表示 | ON |
| 自動スクロール | ON |

### タイムゾーン変換

タイムスタンプはバックエンドでUTCとして保存され、フロントエンドでユーザーのローカルタイムゾーンに変換して表示される。

#### データフロー

```
YouTube API (timestampUsec: マイクロ秒)
    ↓ Backend: chrono::DateTime::from_timestamp()
UTC DateTime (RFC3339形式)
    ↓ Frontend: new Date() + toLocaleTimeString()
ローカルタイムゾーン (HH:MM:SS形式)
```

#### 変換仕様

| レイヤー | 形式 | 例（JST +09:00の場合） |
|---------|------|----------------------|
| YouTube API | マイクロ秒 | `1737020127000000` |
| Backend保存 | RFC3339 (UTC) | `2025-01-16T04:55:27+00:00` |
| Frontend表示 | HH:MM:SS (ローカル) | `13:55:27` |

#### 実装詳細

**Backend (Rust):**
```rust
fn format_timestamp(timestamp_usec: &str) -> String {
    let secs = usec / 1_000_000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0);
    datetime.to_rfc3339()  // UTC RFC3339形式
}
```

**Frontend (TypeScript):**
```typescript
const date = new Date(message.timestamp);  // RFC3339をパース
date.toLocaleTimeString('ja-JP', {         // ローカルタイムゾーンに変換
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
});
```

#### 注意事項

- データベースにはUTCで保存（国際標準に準拠）
- 表示はユーザーのシステム設定に基づくタイムゾーンで変換
- `timestamp_usec`フィールドは元のマイクロ秒精度を保持（必要に応じて使用可能）

#### 表示件数制限

| オプション | 説明 |
|-----------|------|
| 50件 | 軽量表示 |
| 100件 | - |
| 200件 | - |
| 500件 | - |
| 無制限 | デフォルト |

※上限を設定した場合、超過した古いメッセージはアーカイブに移動

#### 実装詳細

- `displayedMessages` = `filteredMessages.slice(-displayLimit)`（displayLimitがnullの場合はfilteredMessagesと同一）
- `filteredMessages` はフィルタのみ適用（ステータスバーの件数表示に使用）
- `messages` は追記と「クリア」だけで変わる（`$state.raw`。足すときは配列ごと差し替え、メッセージ自体は変えない）。
  `filteredMessages` はフィルタが変わらない間、新しく届いた分だけを判定して前回の結果に足す（全件を判定し直すのはフィルタを変えたときと「クリア」のあと）
- アーカイブ = `messages` 配列内に存在するが `displayedMessages` に含まれないメッセージ
- ViewerInfoPanelの過去コメントは `messages` 配列（全件）を参照するため、アーカイブ済みメッセージも表示可能

#### フロントエンドメッセージ上限

フロントエンド（`messages` 配列）に保持するメッセージの件数に上限はない。仮想スクロールにより、大量メッセージでもDOMにはビューポート近辺のみレンダリングされるため、パフォーマンスへの影響は限定的。

#### 仮想スクロール

チャットメッセージの表示にはライブラリ `virtua` による仮想スクロールを使用する。
DOMにはビューポート近辺のメッセージのみレンダリングされ、大量メッセージ（1万件以上）でもスムーズにスクロール可能。

### 配信者向け情報の優先度

配信者が一目で確認すべき情報を優先度順に配置。

| 優先度 | 情報 | 表示位置 |
|-------|------|---------|
| 最高 | メッセージテキスト | 第2行 |
| 最高 | 投稿者名 | 第1行左（色分け） |
| 高 | メンバー/非メンバー | 著者名色 + 背景色 + バッジ |
| 高 | スーパーチャット金額 | ヘッダー行バッジ + 配色 |
| 高 | 初見さん | 第1行（🎉NEWバッジ） |
| 中 | 投稿者アイコン | 第1行左 |
| 中 | 読み仮名 | 第1行（登録時） |
| 中 | バッジ（モデ、認証） | 第1行右 |
| 低 | 配信内コメント回数 | 第1行右 |
| 低 | タイムスタンプ | 第1行左端 |

## 視聴者情報パネル

配信者が視聴者を識別・管理するためのスライドインパネル。

### どの配信者のもとで開くか

視聴者のプロフィールと読み仮名・メモは配信者ごとにある（06_viewer.md「配信者スコープ」）。パネルは、一覧でクリックしたコメントが届いた接続の配信者のもとで視聴者を引き、保存もそこに書く。TTS もメッセージが届いた接続の配信者のもとで読み仮名を引くので、パネルで保存した読み仮名はその接続のコメントの読み上げに使われる。

例: 接続1（配信者A: UCaaa）と接続2（配信者B: UCbbb）に同時に接続している。

| 状況 | 結果 |
|------|------|
| 接続2 に届いた視聴者X（UCxxx）のコメントをクリック | B のもとで X を引く。保存すると B のもとの X に書く（接続2 に X のコメントが届いたら、TTS はこの読み仮名で読む） |
| 接続1 に届いた X のコメントをクリック | A のもとで X を引く |
| パネルを開いたまま、一覧で別の接続に届いたコメントをクリック | クリックしたコメントの接続の配信者で引き直す |
| パネルの「投稿されたコメント」で、別の接続に届いた X のコメントを押す | 一覧のそのコメントへスクロールするだけ。配信者は開いたときのまま |
| パネルを開いたあとでその接続を切断した | 開いたときの配信者のまま。保存もそこに書く |
| クリックしたコメントの接続が切断済み（接続一覧に無い） | パネルを開かない（どの配信者のもとか分からない） |
| クリックしたコメントの接続の配信者 channel_id が分からない | パネルを開かない |

### レイアウト

```
┌──────────────────────────────────┐
│ ▶ 視聴者情報                   ✕ │ ← ヘッダー
├──────────────────────────────────┤
│ [アイコン 56×56]                 │
│ 表示名 (読み仮名)                │
├──────────────────────────────────┤
│ Channel ID: UCxxx...             │
├──────────────────────────────────┤
│ 読み仮名（ふりがな）             │
│ [入力欄]                         │
│ ※視聴者名の横に括弧書きで表示    │
│ [保存] ✓ 保存しました            │
├──────────────────────────────────┤
│ 投稿されたコメント (N件/新着順)   │
│ ┌─────────────────────────────┐ │
│ │ 12:34:56                    │ │
│ │ コメント内容...              │ │
│ │ [💰 ¥500] [メンバーバッジ]   │ │
│ └─────────────────────────────┘ │
│  (最大高300px, スクロール可)     │
└──────────────────────────────────┘
```

### パネルスタイル

| 項目 | 値 |
|-----|-----|
| 幅 | 320px |
| 位置 | 右側固定 |
| 背景 | `var(--bg-surface-1)` |
| ヘッダー背景 | `var(--bg-surface-2)` |
| 入力欄背景 | `var(--bg-surface-3)` |
| アニメーション | slideIn 0.25s ease-out（右から左へ） |
| z-index | 50 |
| シャドウ | `-4px 0 12px rgba(0,0,0,0.3)` |

### 構成要素

| 要素 | サイズ | 色 | 備考 |
|------|--------|-----|------|
| ヘッダータイトル | 20px | `var(--text-primary)` | "視聴者情報" |
| 閉じるボタン | 16px | `var(--text-secondary)` | "✕" |
| アイコン | 56×56px | - | 円形、なければSVGユーザーアイコン |
| 視聴者名 | 18px | `var(--text-primary)` | font-weight: 600 |
| 読み仮名表示 | 16px | `var(--accent)` | 括弧内 |
| チャンネルID | 13px | `var(--text-muted)` | word-break適用 |
| ラベル | 16px | `var(--text-primary)` | - |
| 入力欄 | - | `var(--text-primary)` | padding: 12px 14px |
| 保存ボタン | 16px | `var(--text-inverse)` | 背景: `var(--accent)` |
| 保存メッセージ | 15px | `var(--success)` | "保存しました" |
| コメント数 | 16px | `var(--text-primary)` | "(N件/新着順)" |

### 読み仮名機能

配信者がTTS読み上げ時の発音を改善するための機能。

| 項目 | 仕様 |
|-----|------|
| 入力形式 | テキスト |
| プレースホルダー | "例: やまだ たろう" |
| 保存タイミング | 保存ボタン押下時 |
| 表示位置 | メッセージの著者名の横に括弧書き |
| 空文字の扱い | Noneとして保存（読み仮名なし） |
| 開いてから読み込みが終わるまで | 読み仮名・メモの欄と保存ボタンは押せない（読み込み結果が入力中の内容を上書きしないように） |
| 読み込み中に別の視聴者を開いた | 前の視聴者の読み込み結果は捨てる。フォームの内容と保存先は、いま開いている視聴者のもの（例: A を開いてすぐ B を開き、A の応答があとから届いても、読み仮名は B のまま。保存すると B に書く） |

**保存フロー:**
```
1. 読み仮名を入力
2. 保存ボタンクリック
3. ViewerCustomInfoを作成
4. DBにUpsert（存在すれば更新、なければ新規作成）
5. "保存しました"メッセージ表示（3秒後に自動消去）
```

### 過去コメント表示

当該視聴者のコメントを新着順で表示。
パネルを開いている間に届いたその視聴者のコメントも、そのまま一覧に加わる。

#### データソース

| 優先度 | ソース | 説明 |
|-------|-------|------|
| 1 | メモリバッファ | 現在表示中のメッセージ |
| 2 | アーカイブ | 表示件数制限で移動したメッセージ |

**対象範囲:**
- 現在のプロセスで接続した配信のコメント（フロントエンドの `messages` 配列に存在するもの）

**表示要素:**

| 要素 | サイズ | 色 |
|------|--------|-----|
| タイムスタンプ | 13px | `#aaa` |
| コメント内容 | 15px | `#fff` |

**メッセージタイプバッジ:**

| タイプ | アイコン | 背景色 | テキスト色 |
|-------|---------|--------|----------|
| SuperChat | 💰 + 金額 | `bg-yellow-400/20` | `text-yellow-400` |
| SuperSticker | 🎨 + 金額 | `bg-purple-400/20` | `text-purple-400` |
| Membership（新規） | ⭐ 新規メンバー | `var(--success-subtle)` | `var(--success)` |
| Membership（マイルストーン） | 🎉 + Xヶ月継続 | `var(--info-subtle)` | `var(--info)` |
| MembershipGift | 🎁 + X件ギフト | `bg-pink-400/20` | `text-pink-400` |
| System | ℹ️ システム | `var(--bg-surface-3)` | `var(--text-muted)` |

**スクロール:**
- 最大高: 300px
- overflow-y: auto
- コメント間ギャップ: 8px

**コメントクリック時の挙動:**

過去コメントをクリックすると、メインチャットの該当メッセージまでスクロールしてハイライト表示する。

| 処理 | 詳細 |
|-----|------|
| 自動スクロール無効化 | クリック時に自動スクロールをOFFにする |
| スクロール実行 | `scrollIntoView({ behavior: 'smooth', block: 'center' })` |
| ハイライト表示 | 該当メッセージを選択状態にする |

**スクロール仕様:**

| 項目 | 値 |
|-----|-----|
| 特定方法 | `data-message-id` 属性でDOM要素を特定 |
| アニメーション | smooth（スムーズスクロール） |
| 配置位置 | center（画面中央） |

**ハイライト仕様:**

| 項目 | 値 |
|-----|-----|
| 枠線 | 2px solid `var(--accent)` |
| グロー | box-shadow: 0 0 8px rgba(56,189,248,0.4) |
| 持続時間 | 永続（別のメッセージをクリックするまで） |

**処理フロー:**
```
1. 過去コメントをクリック
2. 自動スクロールを無効化
3. SelectedViewerを更新（クリックしたメッセージで）
4. JavaScriptでメインチャットをスクロール
5. 該当メッセージをハイライト表示
```

**エッジケース:**

| ケース | 動作 |
|-------|------|
| メッセージがアーカイブ済み | スクロール失敗（DOM要素が存在しない） |
| メッセージがフィルターで非表示 | スクロール失敗（DOM要素が存在しない） |
| メッセージが表示範囲内 | 正常にスクロール＆ハイライト |

### データモデル

#### SelectedViewer

パネルに表示する視聴者情報。

```rust
pub struct SelectedViewer {
    pub broadcaster_channel_id: String,    // 配信者ID
    pub viewer_channel_id: String,         // 視聴者ID
    pub display_name: String,              // 表示名
    pub message: GuiChatMessage,           // クリックされたメッセージ
    pub custom_info: Option<ViewerCustomInfo>,
}
```

#### ViewerCustomInfo

視聴者のカスタム情報（配信者ごとに管理）。

```rust
pub struct ViewerCustomInfo {
    pub id: Option<i64>,
    pub broadcaster_channel_id: String,    // 配信者ごとのスコープ
    pub viewer_channel_id: String,
    pub reading: Option<String>,           // 読み仮名
    pub notes: Option<String>,             // メモ（将来用）
    pub custom_data: Option<String>,       // JSON形式拡張データ（将来用）
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
```

### 永続化

#### テーブルスキーマ

```sql
CREATE TABLE IF NOT EXISTS viewer_custom_info (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    broadcaster_channel_id TEXT NOT NULL,
    viewer_channel_id TEXT NOT NULL,
    reading TEXT,
    notes TEXT,
    custom_data TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(broadcaster_channel_id, viewer_channel_id)
);

CREATE INDEX idx_viewer_custom_info_lookup
    ON viewer_custom_info(broadcaster_channel_id, viewer_channel_id);
```

#### 配信者ごとの分離

同一視聴者でも配信者ごとに異なる読み仮名を設定可能。

| 配信者 | 視聴者 | 読み仮名 |
|-------|-------|---------|
| 配信者A | 視聴者X | "たなか" |
| 配信者B | 視聴者X | "タナカさん" |

### バックエンドコマンド

| コマンド | 入力 | 出力 | 説明 |
|---------|------|------|------|
| `viewer_get_profile` | `broadcaster_id, channel_id` | `Option<GuiViewerProfile>` | 視聴者プロフィールを取得 |
| `viewer_get_custom_info` | `viewer_profile_id: i64` | `Option<ViewerCustomInfo>` | 視聴者カスタム情報を取得 |
| `viewer_upsert_custom_info` | `viewer_profile_id, reading, notes, ...` | `()` | カスタム情報を保存（Upsert） |
| `viewer_delete` | `viewer_profile_id: i64` | `bool` | 視聴者を削除 |

詳細は[視聴者管理機能仕様](06_viewer.md)を参照。

### データ取得フロー

視聴者カスタム情報はプリロードキャッシュではなく、パネル表示時にオンデマンドで取得する。

```
メッセージクリック
  ↓
broadcaster_id = chatStore.broadcasterChannelIdOf(message)（null ならパネルを開かない）
  ↓
viewer_get_profile(broadcaster_id, channel_id)
  ↓
viewer_get_custom_info(viewer_profile_id)
  ↓
ViewerInfoPanel に表示
  ↓
保存時に viewer_upsert_custom_info を呼び出し
```

### インタラクション

#### 開く

メッセージクリック → ChatDisplay.svelte の `selectedViewer` をセット（ローカル状態）

#### 閉じる

- ヘッダーの「✕」ボタンクリック
- `selectedViewer = null`

※モードレス（背後のチャット表示は操作可能）

### 利用シーン

| シーン | 操作 |
|-------|------|
| 読み仮名設定 | コメントクリック → 読み仮名入力 → 保存 |
| 常連確認 | コメントクリック → 過去コメント一覧を確認 |
| 投げ銭履歴確認 | コメントクリック → SuperChatバッジで金額確認 |

## 永続化

### セッション

| テーブル | 用途 |
|---------|------|
| `sessions` | 接続セッション情報 |
| `messages` | チャットメッセージ |

### セッションデータ

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    stream_url TEXT,
    stream_title TEXT,
    broadcaster_channel_id TEXT,
    broadcaster_name TEXT,
    started_at TEXT,
    ended_at TEXT,
    total_messages INTEGER,
    total_revenue REAL,
    video_id TEXT  -- 同じ配信のセッションを引く（列の詳細は 08_database.md）
);
```

### メッセージデータ

```sql
CREATE TABLE messages (
    id TEXT PRIMARY KEY,  -- YouTube message ID（重複排除）
    session_id TEXT,
    timestamp_usec TEXT,
    author TEXT,
    author_channel_id TEXT,
    content TEXT,
    message_type TEXT,
    amount TEXT,
    is_member INTEGER,
    raw_json TEXT
);
```

## エラーハンドリング

### 接続エラー

| エラー | 発生条件 | 動作 |
|-------|---------|------|
| Invalid URL | URLからビデオIDを抽出できない | 接続失敗を返却 |
| Page Fetch Failed | YouTubeページの取得に失敗 | 接続失敗を返却 |
| API Key Not Found | HTMLからAPI Keyを抽出できない | 接続失敗を返却 |
| Continuation Not Found | Continuation Tokenを抽出できない | 接続失敗を返却 |
| Member Only | メンバー限定配信で認証なし | 接続失敗を返却 |

### ポーリングエラー

| エラー | 動作 |
|-------|------|
| API応答エラー（2xx 以外・JSON でない） | warnログ、次のポーリングで再試行（「取得に失敗したとき」） |
| DB保存エラー | warnログ、メッセージ処理は継続 |
| ネットワークエラー・タイムアウト | warnログ、次のポーリングで再試行（「取得に失敗したとき」） |
| 15回続けて失敗 | 切断し、エラーを表示 |

### HTMLパース失敗時のデバッグ

重要な情報が抽出できなかった場合、HTMLを一時ファイルに保存する。

```
%TEMP%/liscov_debug_html_{reason}.txt
```

## 運用ケース

### アーカイブ再生

| 項目 | 動作 |
|-----|------|
| 判定 | YouTubeページのメタデータから`is_replay`を検出 |
| 表示 | UIにアーカイブ再生中であることを表示 |
| 動作 | 通常のライブと同様にメッセージを取得（再生時のチャットリプレイ） |

### 接続中の認証情報変更

| 操作 | 動作 |
|-----|------|
| 接続中にログアウト | 既存接続は旧認証情報を保持、メンバー限定チャット取得は継続可能 |
| 接続中に再ログイン | 既存接続には影響なし。次回`connect_to_stream`から新認証を使用 |
| 切断後に再接続 | 最新の認証情報を読み込んで接続 |

### メンバーシップメッセージの判定

| パターン | 判定 |
|---------|------|
| 新規メンバー | `milestone_months: None` |
| マイルストーン（n か月） | `milestone_months: Some(n)` |
| ギフト配布 | `message_type: membership_gift` + `gift_count` |
| ギフト受け取り | 新規メンバーとして処理 |

#### マイルストーン月数の抽出パターン

```
日本語: "(\d+)\s*か月", "メンバー歴\s*(\d+)"
英語: "(\d+)\s*month", "member\s+for\s+(\d+)"
```

#### ギフト数の抽出パターン

```
日本語: "(\d+)\s*人に"
英語: "[Gg]ifted\s+(\d+)", "(\d+)\s+membership"
```
