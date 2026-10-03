<script lang="ts">
  // ErrorBoundary.test.ts 用: 外側の状態と、キーを重複させられる each を並べる
  import ErrorBoundary from '$lib/components/ui/ErrorBoundary.svelte';

  let items = $state(['item-a']);
  let outer = $state(0);
</script>

<button onclick={() => (items = ['item-a', 'item-a'])}>重複させる</button>
<button onclick={() => (items = ['item-b'])}>直す</button>
<button onclick={() => outer++}>外側を数える</button>
<p>外側: {outer}</p>

<ErrorBoundary name="テスト領域">
  {#each items as item (item)}
    <span>{item}</span>
  {/each}
</ErrorBoundary>
