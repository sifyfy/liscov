<script lang="ts">
  /**
   * 方向性の試作 B「分割型」(/design-proto/b?scene=...)
   *
   * 軸: 画面を上下に分け、上に「残す」ためのパネルを常に出しておく。操作は何も要らない。
   * - 上 (高さの 2/5): パネルを横に並べる。通常は「スパチャ等 (配信全体、時刻順)」と「新規さん」の 2 枚、参加型では「参加」が加わる
   *   (パネルの組み合わせを場面ごとに選べる仕組みを想定。OBS のパネルの配置に近い)
   * - 下: コメントの流れ。スパチャ等は流れの中では 1 行に畳む (中身は上のパネルで読む)
   * - 配信の最後のお礼: 上のスパチャ等のパネルがそのまま時刻順の一覧なので、操作ゼロ
   * - 状態は下端の 1 行
   */
  import { page } from '$app/state';
  import Row from '../Row.svelte';
  import Status from '../Status.svelte';
  import Party from '../Party.svelte';
  import { feedOf, newcomersOf, sceneOf, stickToBottom, supportersOf } from '../data';

  const scene = sceneOf(page.url);
  const feed = feedOf(scene);
  const supporters = supportersOf(feed);
  const newcomers = newcomersOf(feed);
</script>

{#snippet panelTitle(label: string, count: number)}
  <h2 class="flex h-7 shrink-0 items-center gap-2 px-3 text-label">
    {label}<span class="text-caption text-fg-2 tabular-nums">{count}</span>
  </h2>
{/snippet}

<main class="flex h-screen flex-col bg-surface font-ui text-fg-1" style="font-size: 13px">
  <div class="flex h-2/5 shrink-0 divide-x divide-outline-variant border-b border-outline bg-surface-low">
    <section class="flex min-w-0 flex-3 flex-col" aria-label="スパチャ等">
      {@render panelTitle('スパチャ等', supporters.length)}
      <ol class="min-h-0 flex-1 overflow-y-auto" {@attach stickToBottom}>
        {#each supporters as m (m.id)}
          <li><Row message={m} compact avatar={false} /></li>
        {/each}
      </ol>
    </section>
    <section class="flex min-w-0 flex-2 flex-col" aria-label="新規さん">
      {@render panelTitle('新規さん', newcomers.length)}
      <ol class="min-h-0 flex-1 overflow-y-auto">
        {#each newcomers as m (m.id)}
          <li><Row message={m} avatar={false} /></li>
        {/each}
      </ol>
    </section>
    {#if scene === 'party'}
      <section class="flex min-w-0 flex-2 flex-col overflow-y-auto" aria-label="参加">
        <Party layout="column" />
      </section>
    {/if}
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto" {@attach stickToBottom}>
    {#each feed as m (m.id)}
      <Row message={m} compact />
    {/each}
  </div>

  <Status class="h-6 border-t border-outline-variant" />
</main>
