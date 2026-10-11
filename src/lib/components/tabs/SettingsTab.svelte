<script lang="ts">
  /**
   * 設定のタブ。上に項目のタブを横に並べ、下に中身を出す (540 幅でも中身の幅を狭めない)
   */
  import { AuthSettings, TtsSettings, RawResponseSettings } from '$lib/components/settings';

  type SettingsSubTab = 'auth' | 'tts' | 'raw' | 'theme';

  // 初期の項目 (ログインの異常から開くときは 'auth')
  let { initialTab = 'auth' }: { initialTab?: SettingsSubTab } = $props();

  // 開いた項目。initialTab が変わったら合わせ直し、押したら書き換える
  let activeSettingsTab = $derived<SettingsSubTab>(initialTab);

  const settingsTabs = [
    { id: 'auth', label: 'YouTube認証' },
    { id: 'tts', label: 'TTS読み上げ' },
    { id: 'raw', label: '生レスポンス保存' },
    { id: 'theme', label: 'UIテーマ' },
  ] as const satisfies readonly { id: SettingsSubTab; label: string }[];
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-hidden bg-surface">
  <nav aria-label="設定の項目" class="flex shrink-0 flex-wrap gap-1 border-b border-outline-variant bg-surface-low px-2 py-1.5">
    {#each settingsTabs as item (item.id)}
      <button
        type="button"
        onclick={() => (activeSettingsTab = item.id)}
        aria-current={activeSettingsTab === item.id ? 'page' : undefined}
        class="rounded-control px-3 py-1 text-label whitespace-nowrap {activeSettingsTab === item.id
          ? 'bg-accent/15 text-accent'
          : 'text-fg-2 hover:bg-fg-1/8'}"
      >
        {item.label}
      </button>
    {/each}
  </nav>
  <div class="min-h-0 flex-1 overflow-y-auto">
    <div class="max-w-2xl">
      {#if activeSettingsTab === 'auth'}
        <AuthSettings />
      {:else if activeSettingsTab === 'tts'}
        <TtsSettings />
      {:else if activeSettingsTab === 'raw'}
        <RawResponseSettings />
      {:else if activeSettingsTab === 'theme'}
        <!-- ThemeSettings は動的インポート（遅延ロード）で読み込む -->
        {#await import('$lib/components/settings/ThemeSettings.svelte') then module}
          <module.default />
        {/await}
      {/if}
    </div>
  </div>
</div>
