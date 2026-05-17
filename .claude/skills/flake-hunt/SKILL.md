---
name: flake-hunt
description: This skill should be used when investigating Rust cargo test intermittent failures (flaky tests). Trigger phrases include "cargo test がたまに失敗", "flaky test 調査", "テストが時々失敗", "間欠的な test 失敗", "テストの再実行で通る", "race condition test", "cargo test FAILED 1 件だけ", "テストのフレーク", "flake hunt", "flake investigation", "this test fails intermittently", "intermittent test failure", "test passes on retry", "address already in use in tests". Provides a four-phase methodology with anti-patterns to avoid and a minimal evidence-preserving harness.
---

# flake-hunt: Rust cargo test 間欠的失敗の調査

## 鉄則 (これを破ると毎回同じ罠に落ちる)

1. **生ログを必ずファイルに保存する。フィルタは保存後に行う。**
   `cargo test ... | grep ... | head -N` は failure 詳細を破棄する最頻パターン。失敗テスト名 (`---- name stdout ----` ブロック) と panic location を失う。
2. **「構造的に脆弱」と「原因」は別。** 怪しいコードを見つけても、再現で確証してから修正する。仮説ベースで修正すると別の真原因を見逃す。
3. **未再現の状態で修正すると、修正の効果を測れない。** 反証も検証もできず、信頼できない fix が積み上がる。

## 4 Phase ワークフロー

### Phase 0: 仕組み化 (証拠保全の不可逆化)

> ⚠️ **核心制約: Claude 自身が `cargo test` の生 stdout を context に直接取り込まない**。サマリ 1 行のみ受け取り、必要な失敗ログだけ Read で開く。これを破ると過去2回踏んだ「grep で証拠喪失」を再演する。

`scripts/flake-hunt.sh` を使う。生ログを必ず `.tmp/flake-runs/<tag>/run-N.log` に保存し、後段でフィルタする。

```bash
# 全テストバイナリ (推奨: --lib 単独では再現しないバグも捕捉できる、A3 参照)
bash .claude/skills/flake-hunt/scripts/flake-hunt.sh 20 allbins -- \
  --manifest-path src-tauri/Cargo.toml

# lib 単独 hunt (低レアフレークの bounded 化に有効)
bash .claude/skills/flake-hunt/scripts/flake-hunt.sh 100 libhunt -- \
  --manifest-path src-tauri/Cargo.toml --lib
```

呼び出し側はサマリの1行だけ受け取る (`TOTAL: 20 iters, 3 failed, logs in .tmp/...`)。失敗があれば対象ログを後から読む。

### Phase 1: 再現条件の絞り込み

「再現できない flake」は存在しない、「条件を当てていないだけ」だと仮定して、条件を体系的に試す。

| 条件 | 目的 | 例 |
|---|---|---|
| target を変える | バイナリ固有か否か切り分け | `--lib` / `--test foo` / 引数無し (全バイナリ) |
| 並列度を変える | intra-binary 並列起因か診断 | `-- --test-threads=1` で 0 になるか |
| 前置処理を入れる | 環境負荷や FS state が引き金か | `--pre-cmd "pnpm test:coverage >/dev/null"` |
| iter 数を増やす | 出現率 < 1/N の場合の bounded 化 | N=100, 500 |

各条件で 20 iter を目安にループ。最初に失敗が出た条件で Phase 2 へ。

**重要な観測**: cargo test を **`--lib` 無し**にすると、lib 単体では再現しないバグが integration test 群とのプロセス間相互作用 (port, file, env, keyring) で再現することがある。`--lib` だけで諦めない。

### Phase 2: 証拠収集 → Phase 3 前に advisor 相談

失敗が出たら `findings.txt` を読み、以下を整理:
- 失敗テスト名 (`---- test_X stdout ----` から抽出)
- panic location (`panicked at file:line` から抽出)
- 失敗テスト群が複数バイナリにまたがるか単一か
- 出現率 (X/N)

**Phase 3 で修正に入る前に必ず advisor() に相談**する。advisor は session の全文脈を見るので、別の仮説を提示してくれることがある。修正方針を独断で決めない。

### Phase 3: 診断 → 仮説検証 → 修正

advisor の方針を踏まえ、仮説を**検証ステップ付き**で試す:

- 「intra-binary 並列が原因」仮説 → `-- --test-threads=1` で 0 失敗になるか確認
- 「特定 test のみ脆弱」仮説 → 該当 test 単独実行 (`--test X testname`) と他 test 並走を比較
- 「OS resource (port/file/env) 競合」仮説 → そのリソース固有の固定値 (port range, hardcoded path) を unique 化すれば消えるか確認

**修正は検証で原因が裏付けされた後に行う。** 仮説段階で修正しない。

### Phase 4: 検証

修正後、Phase 1 で発火した条件で `flake-hunt.sh` N=20 以上を再実行。

- 0/N 失敗 → fix 完了。次のステップ (commit, ledger 記録) へ。
- 1件以上失敗が残る → 原因仮説が不完全。Phase 2 へ戻り再診断。

## アンチパターン集 (体験的に踏んだ罠)

### A1. `cargo test ... | grep "^test result" | head -N`
失敗時の `failures:` ブロックと `---- name stdout ----` を破棄する。**1度やったら 2度目もやる確率が極めて高い** (reflex で書いてしまう)。回避: 必ず `> file` してから `grep` する。

### A2. 構造仮説バイアス
「この関数は明らかにタイミング依存で脆弱だ → ここが原因に違いない」と決めつけて修正する罠。「脆弱性」と「現在の bug」は別。修正で問題が表面上消えても、別の真原因は残っている可能性がある。
回避: 再現で確証 → advisor 相談 → 修正 → Phase 4 検証 の順序を崩さない。

### A3. `cargo test --lib` ループだけで諦める
lib unit test だけで再現しなくても、`cargo test` (引数無し) なら integration test とのプロセス間相互作用で再現することがある。実際にこのプロジェクトの websocket flake はこの方法でしか再現しなかった (条件 H)。

### A4. `#[serial]` を 1テストだけに付ける
`serial_test` クレートの `#[serial]` は **`#[serial]` 同士でしか直列化しない**。非 serial test は並行実行のまま走るので、port や file を OS 層で奪い合うケースには無力。回避: 共有 OS リソースを掴むテストはバイナリ全テストに `#[serial]` を付けるか、リソース確保を unique 化する。

### A5. 反復回数を底なしに増やす
< 1/200 程度のレアフレークは N=500 でもしばしば再現しない。300 iter で出ない場合は条件が違うと判断し、Phase 1 の条件絞り込みに戻る。あるいは bounded として記録 (`status=open`, severity=low) し別タスクへ持ち越す。

## 退場条件 (時間箱)

- 2時間以内で再現条件が見つからない → bounded 化 (`<1/N` を明示) し、`.tmp/flake-runs/` の harness を残して別 PR / 別セッションへ
- Phase 3 で advisor が「仮説不十分」と判断 → 即停止して相談
- 修正後 Phase 4 で 0/N にならない → 修正取り消し or 別仮説で再診断

## 品質オーナーシップ Plugin との連携

調査結果は `event-append.sh` で vicinity_finding に記録する:

```bash
SCRIPT="${CLAUDE_PLUGINS_QO}/scripts/event-append.sh"  # 環境に応じて解決

# 解決した場合
bash "$SCRIPT" --event-type vicinity_finding \
  --title "[resolved] <test名> flake: root cause=<原因>, fix=<修正概要>, 検証: condX N/N → 0/N" \
  --status closed --location "<file:line>" \
  --severity medium --provenance pre-existing-unknown

# 再現できなかった場合
bash "$SCRIPT" --event-type vicinity_finding \
  --title "[bounded] <test名> flake: <N>+ iter で再現不能 (<1/N)、根本原因未確定" \
  --status open --location "<best-guess location or 'unknown'>" \
  --severity low --provenance pre-existing-unknown
```

## 追加リソース

### 参照ファイル

- **`references/patterns.md`** — Rust テストにおける代表的な flake パターン (P1: intra-binary 並列 / P1a: `#[serial]` の盲点 / P2: TOCTOU port / P3: プロセス間 temp_dir & env 共有 / P4: async timing margin / P5: mtime 解像度 / P6: Windows resource 解放遅延) と、各々の判定・修正方針。診断フローチャート付き

### スクリプト

- **`scripts/flake-hunt.sh`** — 反復実行 + 生ログ保存の最小ハーネス (30行未満)

## 環境

- 想定 shell: bash (Windows では MSYS / Git Bash 経由)
- 出力先: `.tmp/flake-runs/<tag>/` (リポジトリ汚染回避のため `.tmp` 配下推奨)
- 想定プロジェクト: cargo workspace (single crate でも multi crate でも OK)
