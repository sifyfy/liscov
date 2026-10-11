<script lang="ts">
  /**
   * 方向性の試作 C「流れ型」(/design-proto/c?scene=...)。★4 で選ばれた案 (2026-10-08)
   *
   * 軸: 1 本の流れだけで済ませ、上に何も置かない。目は常に下端 (最新) に行く。
   * - スパチャ等・新規さんは流れの中で強く出す (アイコンは出さず、行を詰める)
   * - 流れて画面の外に出たものは、下端の欄に残す。新規さんとスパチャ等は別々の欄で、新規さんを上に置く
   *   (流れた新規さんへの反応は時間が勝負。スパチャ等は最後にお礼の一覧でも拾える。シフィさん、2026-10-09)
   *   - 各欄の中では新しいものを上に置く。新しい 2 件は全文の行、それより前は「● 金額 名前」の短い札を折り返して並べる
   *     (消さない、省略しない、古いほど小さく)
   *   - 各欄の高さは画面の 1/8 まで。超えた分はその欄の中でスクロールする (片方が多くても、もう片方を押し出さない)
   *   (試作では「新しいほうから 24 件より前」を画面の外とみなす。本体では実際に見えているかで決める)
   * - 長い本文は省略せず折り返す (省略された文字を読むには手を動かす必要がある。シフィさん、2026-10-08)
   * - 配信の最後のお礼: 欄の「この配信の一覧」を押す (1 手) と、時刻順の一覧が上に開き、下に最新のコメントが残る
   * - 参加型: 参加中・待機を欄の下に足す。状態は下端の 1 行
   */
  import { page } from '$app/state';
  import type { GuiChatMessage } from '$lib/types/generated/GuiChatMessage';
  import Row from '../Row.svelte';
  import Status from '../Status.svelte';
  import Party from '../Party.svelte';
  import { feedOf, newcomersOf, paidLabelOf, sceneOf, stickToBottom, supportersOf } from '../data';

  const scene = sceneOf(page.url);
  const feed = feedOf(scene);
  const supporters = supportersOf(feed);
  let thanksOpen = $state(scene === 'thanks');

  /** 画面に見えているとみなす件数 (試作の近似) */
  const VISIBLE = 24;
  /** 全文の行で出す件数 (新しいほうから) */
  const FULL = 2;

  const outOfView = new Set(feed.slice(0, -VISIBLE).map((m) => m.id));
  const flowedSupporters = [...supporters].reverse().filter((m) => outOfView.has(m.id));
  const flowedNewcomers = newcomersOf(feed).filter((m) => outOfView.has(m.id));
</script>

{#snippet flowed(label: string, items: readonly GuiChatMessage[])}
  {#if items.length > 0}
    <section class="max-h-1/8 shrink-0 overflow-y-auto border-t border-outline bg-surface-low" aria-label={label}>
      <h3 class="sticky top-0 bg-surface-low px-3 pt-1 text-caption text-fg-2">{label} <span class="tabular-nums">{items.length}</span></h3>
      {#each items.slice(0, FULL) as m (m.id)}
        <Row message={m} compact avatar={false} />
      {/each}
      {#if items.length > FULL}
        <!-- それより前のもの: 短い札を折り返して並べる -->
        <!-- 札の区切りが金額と名前の間と紛れないよう、札の頭に印 (ティアの色の点 / メンバーの点 / 初見) を付け、札の間を広くとる -->
        <p class="flex flex-wrap gap-x-4 gap-y-1 px-3 pt-0.5 pb-1.5 text-chat-meta text-fg-2">
          {#each items.slice(FULL) as m (m.id)}
            {@const paid = paidLabelOf(m)}
            {@const colors = m.metadata?.superchat_colors}
            <span class="whitespace-nowrap">
              {#if paid}
                <span
                  class="mr-0.5 inline-block size-2 rounded-full {colors ? '' : 'bg-member'}"
                  style={colors ? `background: ${colors.header_background}` : ''}
                  aria-hidden="true"
                ></span><span class="tabular-nums">{paid}</span>
              {:else}
                <span class="text-highlight">初見</span>
              {/if}
              <span class="ml-0.5 text-fg-1">{m.author}</span>
            </span>
          {/each}
        </p>
      {/if}
    </section>
  {/if}
{/snippet}

<main class="flex h-screen flex-col bg-surface font-ui text-fg-1" style="font-size: 13px">
  <!-- 配信の最後のお礼: 時刻順の一覧 (上、最大で半分) -->
  {#if thanksOpen}
    <section class="flex max-h-1/2 shrink-0 flex-col border-b border-outline bg-surface-low" aria-label="この配信のスパチャ等">
      <h2 class="flex h-8 shrink-0 items-center px-3 text-label">
        この配信のスパチャ等 <span class="ml-2 text-caption text-fg-2 tabular-nums">{supporters.length} 件・時刻順</span>
        <button
          onclick={() => (thanksOpen = false)}
          class="ml-auto rounded-control px-3 py-1 text-label text-fg-2 hover:bg-fg-1/8 active:bg-fg-1/12">閉じる</button
        >
      </h2>
      <ol class="min-h-0 flex-1 overflow-y-auto pb-1">
        {#each supporters as m (m.id)}
          <li><Row message={m} compact avatar={false} /></li>
        {/each}
      </ol>
    </section>
  {/if}

  <div class="min-h-0 flex-1 overflow-y-auto" {@attach stickToBottom}>
    {#each feed as m (m.id)}
      <Row message={m} avatar={false} />
    {/each}
  </div>

  <!-- 流れて画面の外に出た優先順位 2・1 (欄ごとにスクロール) -->
  {@render flowed('流れた新規さん', flowedNewcomers)}
  {@render flowed('流れたスパチャ等', flowedSupporters)}
  {#if !thanksOpen && supporters.length > 0}
    <button
      onclick={() => (thanksOpen = true)}
      class="h-7 shrink-0 border-t border-outline-variant bg-surface-low px-3 text-left text-label text-fg-2 hover:bg-fg-1/8 active:bg-fg-1/12"
    >
      この配信のスパチャ等の一覧 <span class="tabular-nums">{supporters.length}</span>
    </button>
  {/if}

  {#if scene === 'party'}
    <div class="shrink-0 border-t border-outline-variant"><Party /></div>
  {/if}

  <Status class="h-6 border-t border-outline-variant" />
</main>
