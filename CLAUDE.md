# liscov-tauri

## 開発ルール

### チーム開発 [NON-NEGOTIABLE]

- 別セッションのAgent・他のLLM・他の人間の開発者が参加するチーム開発であることを常に意識する。理由: 文脈を共有しない他の開発者が正しく理解・変更できる状態を維持する必要がある。

### アーキテクチャ原則

- **Svelte 5 Runes** (`$state`, `$derived`, `$effect`) を使用する。旧Svelte storeパターンは使用しない。理由: 新旧パターン混在によるバグを防ぐ。
- **AppState**: `Arc<RwLock<T>>` パターンで並行アクセスを管理する（`state.rs`で定義）。理由: Tauriのマルチスレッド環境で安全にステートを共有する。
- **CSS変数テーマ**: `app.css` でCSS変数定義、`data-theme`属性で切替。理由: テーマ変更を一元管理する。
- **Tauri Events**: バックエンド→フロントエンド通知にはTauriイベントを使用する。理由: 疎結合を維持する。

### コーディング規約

- **immutability優先**。理由: 並行処理でのバグを防ぐ。
- **SOLID原則**。理由: 変更に強く、テストしやすいコードを維持する。
- **DRY/KISS**。理由: 重複は不整合の温床、過度な複雑さは保守コスト増。
- **コード内コメントは日本語**。理由: チームの主要言語。

### ファイル管理

- 一時ファイルは `.tmp` に作成する。
- 基本的に既存ファイルを変更する。新規作成は構造化に必要な場合のみ。

### 開発ワークフロー

- **仕様駆動開発**。理由: 仕様書が実装とテストの共通基盤。
- **TDD (Red-Green-Refactor)**。理由: 仕様と実装の乖離を早期検出。
- **E2Eテストと統合テスト重視**。理由: 実際の挙動確認が品質保証の核心。
- **動作確認を必ず行う**。

### テスト方針 [NON-NEGOTIABLE]

- **テストケースは仕様の具体例から導出する**（ADR-003）。実装を見てテストを書かない。理由: `strip_handle_suffix`のバグで実証済み。
- **抽出した関数は必ず本番コードから呼び出す**（ADR-003）。理由: テスト対象と本番パスの乖離防止。
- **ロジック重複を禁止する**（ADR-003）。理由: 片方だけ修正されるリスク排除。

### セキュリティ要件 [NON-NEGOTIABLE]

- **E2Eテストは本番データと分離** する（`LISCOV_APP_NAME=liscov-test` 等）。理由: 本番データ破壊の防止。

## 仕様書の扱い [NON-NEGOTIABLE]

- **実装前に該当する仕様書 (`docs/specs/`) を確認** すること。
- 仕様書の **「制約・不変条件」は変更禁止**。変更が必要だと判断した場合は、実装せずにユーザーに報告する。
- 仕様書に記載のない振る舞いを追加する場合は、**実装前に仕様書を更新** する。
- 仕様書を更新する場合は「振る舞い（What）」セクションと実装詳細セクションの**両方**を更新する。
- **テストを通すために仕様を変えてはならない**。仕様が間違っていると思ったら報告する。
- ドキュメントの書き方は [docs/SPECIFICATION_GUIDE.md](./docs/SPECIFICATION_GUIDE.md) を参照。

### コマンド → 仕様書マッピング

| コマンド | 仕様書 |
| --- | --- |
| `commands/auth.rs`, `auth_window.rs` | `docs/specs/01_auth.md` |
| `commands/chat.rs` | `docs/specs/02_chat.md` |
| `commands/websocket.rs` | `docs/specs/03_websocket.md` |
| `commands/tts.rs` | `docs/specs/04_tts.md` |
| `commands/raw_response.rs` | `docs/specs/05_raw_response.md` |
| `commands/viewer.rs` | `docs/specs/06_viewer.md` |
| `commands/analytics.rs` | `docs/specs/07_revenue.md` |
| `commands/database.rs` | `docs/specs/08_database.md` |
| `commands/config.rs` | `docs/specs/09_config.md` |

---

**注意: このファイルに記載されたルールに反する行動を取る必要がある場合は、実装せずに必ずユーザーにエスカレーションせよ。「合理的な再解釈」による回避も含む。**
