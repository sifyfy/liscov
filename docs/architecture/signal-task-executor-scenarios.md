# SignalTaskExecutor 非GUI統合シナリオ設計

## 背景
SignalManager に導入した `SignalTaskExecutor` は、Dioxus の `spawn` だけでなく Tokio ランタイム等へ処理を委譲できる汎用実行器なのだ。これを活かすことで、GUI 以外のモード（CLI ツールやバックエンドサービス、統合テスト）でも同じ信号最適化ロジックを再利用できるのだ。ここでは非 GUI シナリオを想定した設計指針・API 利用方法・テスト戦略を整理するのだ。

## 実行器オプションの整理
- `SignalTaskExecutor::dioxus()` : Dioxus の `spawn` を通じて WASM/GUI で動かす既定の実行器なのだ。UI スレッドでの安全な更新を保証するのだ。
- `SignalTaskExecutor::tokio()` : Tokio ランタイムへバッチ処理ループを登録する実装なのだ。CLI やヘッドレス統合テストでの再利用を前提にしているのだ。
- `SignalTaskExecutor::new(|task| ...)` : 任意のランタイムへ結合するためのファクトリなのだ。`Handle::spawn` やカスタムスレッドプールと組み合わせればバックエンド統合にも対応できるのだ。

## GUI 外シナリオの設計方針
### 1. CLI / バッチモード
- `SignalManager::new_with_executor(SignalTaskExecutor::tokio())` を用いると、ヘッドレス処理でも 16ms バッチループとデバウンス戦略を維持できるのだ。既存の Tokio ランタイムを使いたい場合は `SignalTaskExecutor::from_handle(tokio::runtime::Handle::current())` を渡しても良いのだ。
- CLI では標準出力へのイベント反映が多いので、`SignalUpdateRequest` に CLI 向けイベント種別を追加する余地があるのだ（別タスク化）。
- 起動スクリプト側で Tokio ランタイムを初期化し、終了時に `shutdown_background_tasks` を呼び出してメトリクスをフラッシュするのが望ましいのだ。

### 2. サービス / デーモン統合
- 常駐プロセスでは `SignalTaskExecutor::new` によって `tokio::runtime::Handle` や `actix` 系統へ結合する設計が必要なのだ。
- ログ集約やメトリクス連携（Prometheus など）を想定し、`UpdateStats` を `Arc<AtomicU64>` ベースに抽象化して外部エクスポート可能にすることを検討するのだ（TODO）。
- 再接続ループと `StreamEndDetector` が同じランタイムを共有するため、タスク増大時は `SignalTaskExecutor` に優先度付きキューを導入する価値があるのだ。

### 3. 統合テスト
- 非 GUI テストでは `SignalTaskExecutor::tokio()` を使い、`#[tokio::test]` のコンテキストで `SignalManager` を生成するのだ。
- Dioxus 依存を避けるため、`SignalTaskExecutor::new` を使って `tokio::runtime::Builder::new_current_thread().enable_all()` で一致するハンドルを使うとフィードバックループを最小化できるのだ。
- テスト終了時は `SignalManager::force_flush()` を呼び出して、バッチ残タスクを同期的に処理するのだ。CLI/サービス環境でも終了フックとして活用できるのだ。

## API・実装面の提案
1. `SignalTaskExecutor::from_handle(handle: tokio::runtime::Handle)` を提供済みで、既存ランタイムの `Handle` を共有するだけで汎用実行器を得られるのだ。
2. `SignalManager::force_flush()`（または `await_idle()`）を提供し、バッチ更新を同期化できるようにするのだ。
3. `SignalUpdateRequest` に `ExecutionMode`（UI/CLI/Service） を持たせ、将来のロギングやメトリクス設定を切り替えられるようにするのだ。
4. エラー伝播を `tracing` だけに頼らず、呼び出し元へ戻せる仕組み（Result 戻り値やエラーフック）を導入するのだ。

## テスト戦略
- 単体: 既存の `signal_optimization_tests` に加え、Tokio 実行器でのメッセージバッチ動作を検証する `SignalTaskExecutor::tokio()` 専用テストを追加するのだ。
- 統合: CLI シナリオ用の `tests/cli_signal_manager.rs`（仮称）で、疑似チャットデータを流してバッチ処理が一定時間内に完了するか検証するのだ。
- 回帰: GUI / CLI / サービス共用のユースケースを `cargo nextest` 等で並列実行し、ランタイム初期化競合を早期検知するのだ。

## TODO / 後続タスク候補
- [x] `SignalTaskExecutor::from_handle` の実装と単体テストを追加するのだ。
- [x] `SignalManager::force_flush()` を実装し、テストと CLI 利用での同期ポイントにするのだ。
- [ ] `tests/cli_signal_manager.rs`（PoC）を作成して、Tokio 実行器利用時の安定性とメトリクス更新を検証するのだ。
- [ ] Metrics / Logging の抽象化レイヤーを設計し、GUI 以外のチャネルでも同じ統計を取得できるようにするのだ。
