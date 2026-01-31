<script lang="ts">
  import { configStore } from '$lib/stores';
  import type { Theme } from '$lib/types';

  let currentTheme = $derived(configStore.theme);

  async function selectTheme(theme: Theme) {
    await configStore.setTheme(theme);
  }
</script>

<div class="p-6 space-y-6">
  <div>
    <h2 class="text-xl font-semibold text-[var(--text-primary)]" style="font-family: var(--font-heading);">UIテーマ</h2>
    <p class="text-sm text-[var(--text-secondary)] mt-1">アプリケーションの外観テーマを選択</p>
  </div>

  <div class="grid grid-cols-2 gap-4">
    <!-- Dark Theme Card -->
    <button
      onclick={() => selectTheme('dark')}
      class="p-4 rounded-lg border-2 transition-all text-left"
      style="background: var(--bg-surface-2); border-color: {currentTheme === 'dark' ? 'var(--accent)' : 'var(--border-default)'};"
    >
      <!-- Mini preview -->
      <div class="rounded-md overflow-hidden mb-3 border" style="border-color: var(--border-subtle);">
        <div class="h-6" style="background: #161822; border-bottom: 1px solid rgba(255,255,255,0.1);">
          <div class="flex items-center gap-1 px-2 h-full">
            <div class="w-1.5 h-1.5 rounded-full" style="background: #38bdf8;"></div>
            <div class="w-8 h-1.5 rounded" style="background: rgba(255,255,255,0.15);"></div>
          </div>
        </div>
        <div class="p-2 space-y-1" style="background: #0f1117;">
          <div class="h-2 w-3/4 rounded" style="background: #1c1f2e;"></div>
          <div class="h-2 w-1/2 rounded" style="background: #1c1f2e;"></div>
          <div class="h-2 w-2/3 rounded" style="background: #1c1f2e;"></div>
        </div>
      </div>
      <div class="flex items-center justify-between">
        <div>
          <p class="font-medium text-[var(--text-primary)]">ダーク</p>
          <p class="text-xs text-[var(--text-muted)]">目に優しい暗いテーマ</p>
        </div>
        {#if currentTheme === 'dark'}
          <div class="w-5 h-5 rounded-full flex items-center justify-center" style="background: var(--accent);">
            <svg class="w-3 h-3 text-[var(--text-inverse)]" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="3">
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </div>
        {/if}
      </div>
    </button>

    <!-- Light Theme Card -->
    <button
      onclick={() => selectTheme('light')}
      class="p-4 rounded-lg border-2 transition-all text-left"
      style="background: var(--bg-surface-2); border-color: {currentTheme === 'light' ? 'var(--accent)' : 'var(--border-default)'};"
    >
      <!-- Mini preview -->
      <div class="rounded-md overflow-hidden mb-3 border" style="border-color: var(--border-subtle);">
        <div class="h-6" style="background: #ffffff; border-bottom: 1px solid rgba(0,0,0,0.08);">
          <div class="flex items-center gap-1 px-2 h-full">
            <div class="w-1.5 h-1.5 rounded-full" style="background: #0284c7;"></div>
            <div class="w-8 h-1.5 rounded" style="background: rgba(0,0,0,0.08);"></div>
          </div>
        </div>
        <div class="p-2 space-y-1" style="background: #f0f2f5;">
          <div class="h-2 w-3/4 rounded" style="background: #e2e8f0;"></div>
          <div class="h-2 w-1/2 rounded" style="background: #e2e8f0;"></div>
          <div class="h-2 w-2/3 rounded" style="background: #e2e8f0;"></div>
        </div>
      </div>
      <div class="flex items-center justify-between">
        <div>
          <p class="font-medium text-[var(--text-primary)]">ライト</p>
          <p class="text-xs text-[var(--text-muted)]">明るく見やすいテーマ</p>
        </div>
        {#if currentTheme === 'light'}
          <div class="w-5 h-5 rounded-full flex items-center justify-center" style="background: var(--accent);">
            <svg class="w-3 h-3 text-[var(--text-inverse)]" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="3">
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </div>
        {/if}
      </div>
    </button>
  </div>
</div>
