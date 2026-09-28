<script lang="ts">
  // ライブリアクションの累計と勢い（02_chat.md「リアクションメーター」）
  import { chatStore } from '$lib/stores/chat.svelte';
  import { hasReactions, reactionsPerMinute, sortedReactionTotals } from '$lib/utils/reactions';

  // 勢いはリアクションが止まっても下がる必要があるので、1秒ごとに現在時刻を進める
  let nowUsec = $state(Date.now() * 1000);
  $effect(() => {
    const timer = setInterval(() => {
      nowUsec = Date.now() * 1000;
    }, 1000);
    return () => clearInterval(timer);
  });

  let rows = $derived(
    [...chatStore.connections.values()]
      .filter((conn) => hasReactions(conn.reactions))
      .map((conn) => {
        const perMinute = reactionsPerMinute(conn.reactions, nowUsec);
        const peak = Math.max(conn.reactions.peakPerMinute, perMinute);
        return {
          id: conn.id,
          color: conn.color,
          broadcasterName: conn.broadcasterName,
          totals: sortedReactionTotals(conn.reactions),
          perMinute,
          fill: peak > 0 ? perMinute / peak : 0
        };
      })
  );
</script>

{#if rows.length > 0}
  <div class="reaction-meter" data-testid="reaction-meter">
    {#each rows as row (row.id)}
      <div class="reaction-row" data-testid="reaction-row">
        <div class="color-indicator" style="background-color: {row.color}"></div>
        <span class="broadcaster-name">{row.broadcasterName}</span>
        <div class="totals">
          {#each row.totals as [emoji, count] (emoji)}
            <span class="total" data-testid="reaction-total" data-emoji={emoji}>{emoji} {count}</span>
          {/each}
        </div>
        <div class="momentum" title="直近60秒のリアクション数">
          <span class="momentum-label">勢い</span>
          <div class="bar"><div class="bar-fill" style="width: {row.fill * 100}%"></div></div>
          <span class="per-minute" data-testid="reaction-per-minute">{row.perMinute}/分</span>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  /* テーマのCSS変数を使用 */
  .reaction-meter {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 2px 8px;
    background: var(--bg-surface-1);
    border-bottom: 1px solid var(--border-subtle);
  }
  .reaction-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.8em;
    color: var(--text-primary);
    white-space: nowrap;
  }
  .color-indicator {
    width: 4px;
    height: 14px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .broadcaster-name {
    max-width: 8em;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-secondary);
  }
  .totals {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 8px;
    overflow: hidden;
  }
  .momentum {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .momentum-label {
    color: var(--text-secondary);
  }
  .bar {
    width: 48px;
    height: 6px;
    border-radius: 3px;
    background: var(--bg-surface-2);
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
  }
  .per-minute {
    min-width: 3.5em;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
