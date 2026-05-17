# Rust テスト flake パターン集

調査時の仮説立てと修正判断のための既知パターン一覧。各パターンには「症状」「判定方法」「修正方針」をセットで記載する。

## P1. intra-binary 並列実行による OS リソース競合

### 症状
- `cargo test --lib` 単体や `--test X` 単体では再現しないが、`cargo test` (引数無し) や複数バイナリ実行時に低確率で fail
- panic は `Failed to start server: No available ports in range ...` や `Address already in use` など OS resource 系
- `tokio::net::TcpListener::bind`、`std::net::*::bind`、ファイルロック、Windows keyring などが該当

### 判定方法
```bash
# 並列度を 1 にすると消えるか
cargo test --test <binary> -- --test-threads=1
```
これで 0 失敗になるなら intra-binary 並列が原因。

### 修正方針 (優先順)
1. リソース確保ロジック側を **per-test unique** にする (port を fixed range にしない、`bind(0)` の戻り値を即使う、temp path に PID/UUID suffix を付ける)
2. それが難しい場合: 該当バイナリの全 `#[test]` / `#[tokio::test]` に `serial_test::serial` を付与する。**1テストだけに付けても効かない** (P1a 参照)

### P1a. `#[serial]` の盲点
`serial_test` の `#[serial]` は **`#[serial]` 同士でしか直列化しない**。1テストだけに付けると、他の非 `#[serial]` テストは並行実行のまま走り、ポートやファイルを掴み合う。

```rust
// 不十分: 他の tokio::test と並行実行されるので無意味
#[tokio::test]
#[serial]
async fn test_a() { ... }

#[tokio::test]
async fn test_b() { /* test_a と並行に走る */ }
```

修正: 該当バイナリの全 test に `#[serial]` を付ける、または共有キー (`#[serial(websocket)]`) で揃える。

## P2. TOCTOU port 取得

### 症状
- `bind(0)` で「空いているポート」を取得して `drop` し、後で「同じポート」または「近接ポート」を再利用するヘルパが flake の元
- 並行する他テストが間に bind して横取りする

```rust
// 危険なパターン
async fn get_test_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);  // ← この瞬間に他テストがこの port を取れる
    port
}
```

### 判定方法
panic に specific port が含まれる (例: `No available ports in range 64195-64204`)。範囲が 10〜30 ポート程度の窓だと、並行 test 群が連続 bind で食い潰している可能性が高い。

### 修正方針
- 「fallback range が必要なテスト」だけを `#[serial]` で直列化する (リソース仕様に基づく)
- もしくは port を **listener を保持したまま** 使い、必要な数だけ事前に確保してから drop で released する

## P3. プロセス間 temp_dir / env var 共有

### 症状
- 複数の cargo test プロセスが同時走行 (CI matrix, 並列ジョブ, `cargo nextest` 検討時) すると低確率で fail
- 単一プロセス実行 (デフォルト `cargo test`) では再現しない
- panic は `指定されたファイルが見つかりません。 (os error 2)`、`file already exists`、credentials の予期せぬ存在など

```rust
// プロセス間で衝突する典型パターン
fn temp_dir_for_test(name: &str) -> PathBuf {
    std::env::temp_dir().join("liscov_test").join(name)  // ← 全プロセスで同じパス
}

#[test]
fn t() {
    std::env::set_var("LISCOV_APP_NAME", "liscov-test");  // ← env は per-process だが
    // ↓ アプリは LISCOV_APP_NAME → 固定パス を導出するので結果はプロセス間共有
}
```

### 判定方法
```bash
# 同じテストバイナリを N 並列で起動
EXE=target/debug/deps/<binary>-<hash>.exe
for i in $(seq 1 50); do "$EXE" > .tmp/flake-runs/parallel/run-$i.log 2>&1 & done
wait
grep -l "FAILED\|panicked" .tmp/flake-runs/parallel/*.log | wc -l
```

これで多くの失敗が出るなら P3。

### 修正方針
- `tempfile::TempDir` (auto-cleanup + unique path) を使う
- env var ベースで namespace 切る場合は `LISCOV_APP_NAME=liscov-test-${PID}-${RANDOM_HEX}` のような per-process unique 値を使う
- keyring service 名なども同様に unique 化
- `#[serial]` では**プロセス間は守れない** (intra-process lock のみ)

## P4. async timing margin 不足

### 症状
- spawn したタスクが「N ms 以内に処理されているはず」を `tokio::time::sleep(N)` で待つテスト
- CPU 負荷下 / cold cache / Windows scheduler のばらつきで `N` を超過し fail

```rust
// 危険なパターン
manager.start_processing().await;
tokio::time::sleep(Duration::from_millis(300)).await;  // 3 item × 100ms poll = 300ms 理論最小
manager.stop_processing().await;
assert_eq!(calls.len(), 3);  // load 下では < 3 になる
```

### 判定方法
- panic は assertion failure (`assertion `left == right` failed`)
- CPU 負荷をかけて (`yes > /dev/null &` 等) ループしてみると再現率が上がる
- ただし P1〜P3 と違って **single-process 単独実行でも load 次第で fail し得る**

### 修正方針
- **固定 sleep を condition-based polling に置き換える**:

```rust
let deadline = Instant::now() + Duration::from_secs(5);
while Instant::now() < deadline {
    if manager.queue_size().await == 0 { break; }
    tokio::time::sleep(Duration::from_millis(10)).await;
}
// 一周分の余裕で in-flight 完了を待つ
tokio::time::sleep(Duration::from_millis(150)).await;
```

- 「N+α ms 待つ」ではなく「N 件処理されるまで待つ (タイムアウト付き)」が原則

## P5. ファイル mtime 解像度依存

### 症状
- バックアップ並び替え、世代管理、最古/最新判定など mtime に依存する処理
- ファイル間に 50ms 程度の sleep を挟むが、FS の mtime resolution によっては同一値になる
- 結果として sort が undefined order になり「古いはずのファイルが残る」「新しいはずのファイルが消える」

### 判定方法
- panic は assertion (`expected file X to exist`)
- 同じテストを CI (Linux) では出ないが Windows / macOS で出るパターンが多い
- FAT32, exFAT, 一部の NTFS 構成で顕著

### 修正方針
- sleep ベースではなく、**ファイル名やメタ情報に明示的な順序情報** を埋め込む (`backup_20250101_000001.ndjson` のようにシーケンス番号)
- mtime に頼らない実装にする (作成時刻 sidecar、または内容ヘッダー)

## P6. リソース解放遅延 (Windows 特有)

### 症状
- TCP listener / file handle / process を drop した直後に同じリソースを bind/open しようとすると `Access is denied` や TIME_WAIT 期間の binding 失敗
- Linux では通常起きないが Windows で散見

### 判定方法
- Windows でのみ低確率で発生
- 直前のテストが close した resource を直後のテストが触る順序

### 修正方針
- リソース解放後に明示的なリトライ (バックオフ) を入れる
- そもそも resource を共有しない (per-test unique を徹底)

## 診断フローチャート (簡易)

```
flake observed
  └ cargo test --lib のみで再現？
      ├ YES → P4 (timing) or lib 内部の race 疑い
      └ NO  → cargo test (no --lib) で再現？
          ├ YES → どのバイナリで fail?
          │      ├ integration test → P1 / P2 (intra-binary 並列)
          │      └ lib → multi-binary 環境の何かが影響
          └ NO  → 並列プロセス起動 (Phase 1 cond E or 直接 EXE 並列実行) で再現？
              ├ YES → P3 (プロセス間共有)
              └ NO  → bounded 化 (低レア)。harness を残して別タスクへ
```

## 既知の限界

- < 1/300 のレアフレークは 1 セッションでの調査が難しい。harness を残し、他開発者・別セッションで再観測されたときに即トリアージできるようにする
- Windows AV (Defender) のスキャンに伴うファイルロックは AV 設定起因なので、テストコード側で対処できない場合がある。AV exclusion で別 issue として扱う

## 参考: このスキルが生まれた経緯

liscov-tauri プロジェクトで `cargo test 1/435` の lib flake を調査し、約 250 iter で再現不能、`drain_speak_calls` の固定 sleep (P4 パターン) を予防修正した経緯から methodology を抽出。同じ調査で副次的に websocket integration test の `test_port_fallback_when_occupied` flake (P1 + P2 パターン) を発見・修正済。
