<script lang="ts">
  import { AuthSettings, TtsSettings, RawResponseSettings } from '$lib/components/settings';

  type SettingsSubTab = 'auth' | 'tts' | 'raw' | 'theme';

  // 初期サブタブ（外部から指定可能、デフォルトは 'auth'）
  let { initialTab = 'auth' }: { initialTab?: SettingsSubTab } = $props();

  let activeSettingsTab = $state<SettingsSubTab>(initialTab);

  // 設定サブタブ一覧
  const settingsTabs: { id: SettingsSubTab; label: string }[] = [
    { id: 'auth', label: 'YouTube認証' },
    { id: 'tts', label: 'TTS読み上げ' },
    { id: 'raw', label: '生レスポンス保存' },
    { id: 'theme', label: 'UIテーマ' }
  ];

  // 初期タブが変わった場合はリセット（openAuthSettings などによる切り替えに対応）
  $effect(() => {
    activeSettingsTab = initialTab;
  });
</script>

<!-- 設定タブ: サイドバーナビ + コンテンツエリア -->
<div class="flex-1 flex overflow-hidden">
  <!-- 設定サイドバー -->
  <div class="w-48 bg-[var(--bg-surface-1)] border-r p-4 flex-shrink-0" style="border-color: var(--border-subtle);">
    <h3 class="text-xs font-semibold text-[var(--text-muted)] uppercase tracking-wider mb-3">Settings</h3>
    <nav class="space-y-0.5">
      {#each settingsTabs as item}
        <button
          onclick={() => (activeSettingsTab = item.id)}
          class="w-full text-left px-3 py-2 rounded-md text-sm transition-all"
          style={activeSettingsTab === item.id
            ? 'background: var(--accent-subtle); color: var(--accent); font-weight: 500;'
            : 'color: var(--text-secondary);'}
        >
          {item.label}
        </button>
      {/each}
    </nav>
  </div>
  <!-- 設定コンテンツ -->
  <div class="flex-1 overflow-y-auto bg-[var(--bg-base)]">
    <div class="max-w-3xl">
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
