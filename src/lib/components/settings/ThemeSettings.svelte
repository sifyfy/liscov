<script lang="ts">
  import { configStore } from '$lib/stores';
  import type { Theme } from '$lib/types';
  import Icon from '$lib/components/ui/Icon.svelte';

  let currentTheme = $derived(configStore.theme);

  async function selectTheme(theme: Theme) {
    await configStore.setTheme(theme);
  }

  // 見本は本物のトークンを data-theme で切り替えて描く (生の色値で描くと実際の色とずれる)
  const THEMES = [
    { id: 'dark', label: 'ダーク', description: '目に優しい暗いテーマ' },
    { id: 'light', label: 'ライト', description: '明るく見やすいテーマ' },
  ] as const satisfies readonly { id: Theme; label: string; description: string }[];
</script>

<div class="flex flex-col gap-4 p-4 text-fg-1">
  <header class="flex flex-col gap-1">
    <h2 class="text-title">UIテーマ</h2>
    <p class="text-body text-fg-2">アプリケーションの外観テーマを選択</p>
  </header>

  <div class="grid grid-cols-2 gap-3">
    {#each THEMES as theme (theme.id)}
      {@const selected = currentTheme === theme.id}
      <button
        type="button"
        onclick={() => selectTheme(theme.id)}
        aria-pressed={selected}
        class="flex flex-col gap-2 rounded-popup border-2 bg-surface-low p-3 text-left hover:bg-surface-high {selected
          ? 'border-accent'
          : 'border-outline-variant'}"
      >
        <!-- 見本: 上の行とコメントの行 -->
        <div data-theme={theme.id} class="overflow-hidden rounded-control border border-outline-variant bg-surface" aria-hidden="true">
          <div class="flex h-5 items-center gap-1 border-b border-outline-variant bg-surface-low px-2">
            <span class="size-1.5 rounded-full bg-success"></span>
            <span class="h-1.5 w-8 rounded-full bg-fg-3"></span>
          </div>
          <div class="flex flex-col gap-1 p-2">
            <span class="h-1.5 w-3/4 rounded-full bg-fg-1"></span>
            <span class="h-1.5 w-1/2 rounded-full bg-fg-2"></span>
            <span class="h-1.5 w-2/3 rounded-full bg-highlight"></span>
          </div>
        </div>
        <span class="flex items-center justify-between gap-2">
          <span class="flex flex-col">
            <span class="text-label">{theme.label}</span>
            <span class="text-caption text-fg-2">{theme.description}</span>
          </span>
          {#if selected}
            <Icon name="check" size={16} class="shrink-0 text-accent" />
          {/if}
        </span>
      </button>
    {/each}
  </div>
</div>
