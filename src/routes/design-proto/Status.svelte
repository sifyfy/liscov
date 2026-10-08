<script lang="ts">
  /**
   * 優先順位 6〜7 (同接・いいね・チャット率、リアクション) と接続の状態の 1 行。控えめに出す (ブリーフ §2)
   * - 接続中は小さい点だけ。リアクションの絵文字は色を抜く (色つきだと優先順位 1 の色と競う)
   */
  import { formatCount, reactions, stats } from './data';

  let { class: cls = '' }: { class?: string } = $props();
</script>

<div class="flex shrink-0 items-center gap-3 px-3 text-caption text-fg-3 tabular-nums {cls}">
  <span class="size-2 shrink-0 rounded-full bg-success" title="接続中"></span>
  <span>同接 {formatCount(stats.viewers)}</span>
  <span>いいね {formatCount(stats.likes)}</span>
  <span>{stats.chatPerMinute}/分</span>
  <span class="ml-auto flex items-center gap-2">
    {#each reactions.counts as r (r.emoji)}
      <span><span class="grayscale">{r.emoji}</span> {r.count}</span>
    {/each}
    {#if reactions.rising}<span aria-label="上昇中">▲</span>{/if}
  </span>
</div>
