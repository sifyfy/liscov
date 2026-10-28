---
name: flake-hunt
description: This skill should be used when investigating Rust cargo test intermittent failures (flaky tests). Trigger phrases include "cargo test がたまに失敗", "flaky test 調査", "テストが時々失敗", "間欠的な test 失敗", "テストの再実行で通る", "race condition test", "cargo test FAILED 1 件だけ", "テストのフレーク", "flake hunt", "flake investigation", "this test fails intermittently", "intermittent test failure", "test passes on retry", "address already in use in tests". Provides a four-phase methodology with anti-patterns to avoid and a minimal evidence-preserving harness.
---

# flake-hunt: Rust cargo test 間欠的失敗の調査

**このスキルは手順書型が主の混合型。** Phase 0〜4 の流れ・鉄則・退場条件はそのとおりに進める。
Phase 3 の原因の見立てだけは判断が要るので、`references/patterns.md` のパターンと判定を前提に照らして使う。
手順に無い状況になったら「退場条件」へ。

前提条件 (崩れたら手順の外):

- 対象は cargo のワークスペース (単一クレートでも複数でもよい) の `cargo test`
- bash で `scripts/flake-hunt.sh` を回せる (Windows では Git Bash)
- 失敗が間欠的である。毎回落ちるなら flake ではないので、普通のテストの失敗として調べる

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

### Phase 2: 証拠収集 → Phase 3 の前に別の仮説を出させる

失敗が出たら `findings.txt` を読み、以下を整理:
- 失敗テスト名 (`---- test_X stdout ----` から抽出)
- panic location (`panicked at file:line` から抽出)
- 失敗テスト群が複数バイナリにまたがるか単一か
- 出現率 (X/N)

**Phase 3 で修正に入る前に、調査の文脈を持たないサブエージェントに別の仮説を出させる。** 自分で見つけた怪しいコードに引きずられる (A2) のを防ぐため。

- 渡すもの: 上の整理 (失敗テスト名・panic location・バイナリの範囲・出現率)、発火した条件 (Phase 1 の表のどれか)、`references/patterns.md`
- 渡さないもの: 自分の仮説と、怪しいと思ったコード (仮説が引きずられる)
- 頼むこと: 証拠と矛盾しない仮説を複数、それぞれを確かめる実験と一緒に
- 自分の仮説と食い違ったら、両方を Phase 3 の検証にかける。どちらを先に試すか決めきれなければシフィさんに相談する

### Phase 3: 診断 → 仮説検証 → 修正

Phase 2 の仮説を並べ、**検証ステップ付き**で試す:

- 「intra-binary 並列が原因」仮説 → `-- --test-threads=1` で 0 失敗になるか確認
- 「特定 test のみ脆弱」仮説 → 該当 test 単独実行 (`--test X testname`) と他 test 並走を比較
- 「OS resource (port/file/env) 競合」仮説 → そのリソース固有の固定値 (port range, hardcoded path) を unique 化すれば消えるか確認

**修正は検証で原因が裏付けされた後に行う。** 仮説段階で修正しない。

### Phase 4: 検証

修正後、Phase 1 で発火した条件で `flake-hunt.sh` N=20 以上を再実行。

- 0/N 失敗 → fix 完了。次のステップ (commit と「記録」の節) へ。
- 1件以上失敗が残る → 原因仮説が不完全。Phase 2 へ戻り再診断。

## アンチパターン集 (体験的に踏んだ罠)

### A1. `cargo test ... | grep "^test result" | head -N`
失敗時の `failures:` ブロックと `---- name stdout ----` を破棄する。**1度やったら 2度目もやる確率が極めて高い** (reflex で書いてしまう)。回避: 必ず `> file` してから `grep` する。

### A2. 構造仮説バイアス
「この関数は明らかにタイミング依存で脆弱だ → ここが原因に違いない」と決めつけて修正する罠。「脆弱性」と「現在の bug」は別。修正で問題が表面上消えても、別の真原因は残っている可能性がある。
回避: 再現で確証 → 別の仮説を出させる (Phase 2) → 修正 → Phase 4 検証 の順序を崩さない。

### A3. `cargo test --lib` ループだけで諦める
lib unit test だけで再現しなくても、`cargo test` (引数無し) なら integration test とのプロセス間相互作用で再現することがある。実際にこのプロジェクトの websocket flake はこの方法でしか再現しなかった (条件 H)。

### A4. `#[serial]` を 1テストだけに付ける
`serial_test` クレートの `#[serial]` は **`#[serial]` 同士でしか直列化しない**。非 serial test は並行実行のまま走るので、port や file を OS 層で奪い合うケースには無力。回避: 共有 OS リソースを掴むテストはバイナリ全テストに `#[serial]` を付けるか、リソース確保を unique 化する。

### A5. 反復回数を底なしに増やす
< 1/200 程度のレアフレークは N=500 でもしばしば再現しない。300 iter で出ない場合は条件が違うと判断し、Phase 1 の条件絞り込みに戻る。あるいは bounded として記録 (下の「記録」) し別タスクへ持ち越す。

## 退場条件 (時間箱)

- 2時間以内で再現条件が見つからない → bounded 化 (`<1/N` を明示) し、`.tmp/flake-runs/` の harness を残して別 PR / 別セッションへ
- Phase 3 で、どの仮説も検証で裏付けられない → 即停止してシフィさんに相談
- 修正後 Phase 4 で 0/N にならない → 修正取り消し or 別仮説で再診断

## 記録

後から同じ flake を調べる人が、条件と結論から始められるようにする。書く項目は解決・未解決で共通:
テスト名・発火した条件・出現率 (X/N)・原因 (未確定ならそう書く)・修正・検証 (`条件X N/N → 0/N`)・`.tmp/flake-runs/<tag>/` の場所。

- 解決した → 修正のコミットメッセージか PR の説明に書く
- 再現できず持ち越す (bounded) → タスク記録 (`~/tasks/`) の handoff・log に書く。liscov は public なので、issue にするかはシフィさんに確かめてから

## 追加リソース

### 参照ファイル

- **`references/patterns.md`** — Rust テストにおける代表的な flake パターン (P1: intra-binary 並列 / P1a: `#[serial]` の盲点 / P2: TOCTOU port / P3: プロセス間 temp_dir & env 共有 / P4: async timing margin / P5: mtime 解像度 / P6: Windows resource 解放遅延) と、各々の判定・修正方針。診断フローチャート付き

### スクリプト

- **`scripts/flake-hunt.sh`** — 反復実行 + 生ログ保存の最小ハーネス (30行未満)

## 環境

- 出力先: `.tmp/flake-runs/<tag>/` (リポジトリ汚染回避のため `.tmp` 配下推奨)
- shell と対象のプロジェクトは冒頭の前提条件
