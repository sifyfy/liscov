<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { chatStore, configStore, websocketStore, viewerStore, analyticsStore, ttsStore, authStore } from '$lib/stores';
  import { ChatDisplay, FilterPanel, InputSection } from '$lib/components/chat';
  import { ViewerManagement } from '$lib/components/viewer';
  import { RevenueDashboard, ExportPanel } from '$lib/components/analytics';
  import { AuthSettings, TtsSettings, RawResponseSettings } from '$lib/components/settings';
  import { AuthIndicator, StorageErrorDialog } from '$lib/components/auth';
  import Icon from '$lib/components/ui/Icon.svelte';

  type Tab = 'chat' | 'viewers' | 'analytics' | 'settings';
  type SettingsTab = 'auth' | 'tts' | 'raw' | 'theme';
  let activeTab = $state<Tab>('chat');
  let activeSettingsTab = $state<SettingsTab>('auth');
  let showStorageErrorDialog = $state(false);

  const tabInfo: Record<Tab, { icon: 'chat' | 'users' | 'chart' | 'settings'; label: string; shortLabel: string }> = {
    chat: { icon: 'chat', label: 'Chat Monitor', shortLabel: 'Chat' },
    viewers: { icon: 'users', label: 'Viewers', shortLabel: 'Viewers' },
    analytics: { icon: 'chart', label: 'Analytics', shortLabel: 'Analytics' },
    settings: { icon: 'settings', label: 'Settings', shortLabel: 'Settings' }
  };

  let broadcasterId = $derived(chatStore.broadcasterChannelId || '');
  let hasStorageError = $derived(authStore.storageError !== null);

  onMount(async () => {
    await configStore.load();
    chatStore.initDisplaySettings();
    await authStore.refreshStatus();
    if (authStore.storageError) {
      showStorageErrorDialog = true;
    }
    if (authStore.isAuthenticated) {
      authStore.checkSessionValidity();
    }
    await chatStore.setupEventListeners();
    await websocketStore.init();
  });

  function openAuthSettings() {
    activeTab = 'settings';
    activeSettingsTab = 'auth';
  }

  onDestroy(() => {
    chatStore.cleanup();
  });
</script>

<div class="h-screen flex flex-col overflow-hidden bg-[var(--bg-base)]">
  <!-- Header -->
  <header class="flex items-center justify-between px-4 py-2 flex-shrink-0 bg-[var(--bg-surface-1)] border-b" style="border-color: var(--border-default);">
    <!-- Left: Tab info + status -->
    <div class="flex items-center gap-3 min-w-0">
      <div class="flex items-center gap-2">
        <Icon name={tabInfo[activeTab].icon} size={18} class="text-[var(--accent)]" />
        <h1 class="text-sm font-semibold text-[var(--text-primary)]" style="font-family: var(--font-heading);">
          {tabInfo[activeTab].label}
        </h1>
      </div>

      <!-- Status indicators -->
      <div class="flex items-center gap-2 ml-2">
        <AuthIndicator onclick={openAuthSettings} />
        <div class="flex items-center gap-1.5 px-2 py-1 rounded-md bg-[var(--bg-surface-2)]">
          <div
            class="w-1.5 h-1.5 rounded-full"
            style="background: {chatStore.isConnected ? 'var(--success)' : 'var(--text-muted)'};"
          ></div>
          <span class="text-xs text-[var(--text-secondary)]">
            {chatStore.isConnected ? '接続中' : '未接続'}
          </span>
        </div>
        {#if websocketStore.isRunning}
          <div class="flex items-center gap-1.5 px-2 py-1 rounded-md bg-[var(--bg-surface-2)]">
            <div class="w-1.5 h-1.5 rounded-full" style="background: var(--info);"></div>
            <span class="text-xs text-[var(--text-secondary)]" style="font-family: var(--font-mono);">
              WS:{websocketStore.actualPort}({websocketStore.connectedClients})
            </span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Right: Tab navigation -->
    <nav class="flex gap-0.5 flex-shrink-0 p-0.5 rounded-lg bg-[var(--bg-surface-2)]">
      {#each (['chat', 'viewers', 'analytics', 'settings'] as const) as tab}
        <button
          onclick={() => (activeTab = tab)}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all"
          style={activeTab === tab
            ? 'background: var(--accent-subtle); color: var(--accent);'
            : 'color: var(--text-muted);'}
        >
          <Icon name={tabInfo[tab].icon} size={14} />
          <span>{tabInfo[tab].shortLabel}</span>
        </button>
      {/each}
    </nav>
  </header>

  <!-- Main content area -->
  <main class="flex-1 flex flex-col overflow-hidden">
    {#if activeTab === 'chat'}
      <div class="bg-[var(--bg-surface-1)] border-b" style="border-color: var(--border-subtle);">
        <InputSection />
      </div>
      <FilterPanel />
      <div class="flex-1 overflow-hidden">
        <ChatDisplay />
      </div>
    {:else if activeTab === 'viewers'}
      <div class="flex-1 p-4 bg-[var(--bg-base)] overflow-y-auto">
        <ViewerManagement broadcasterId={broadcasterId || undefined} />
      </div>
    {:else if activeTab === 'analytics'}
      <div class="flex-1 p-6 overflow-y-auto bg-[var(--bg-base)]">
        <div class="max-w-6xl mx-auto grid grid-cols-1 lg:grid-cols-3 gap-6">
          <div class="lg:col-span-2">
            <RevenueDashboard />
          </div>
          <div>
            <ExportPanel />
          </div>
        </div>
      </div>
    {:else if activeTab === 'settings'}
      <div class="flex-1 flex overflow-hidden">
        <!-- Settings sidebar -->
        <div class="w-48 bg-[var(--bg-surface-1)] border-r p-4 flex-shrink-0" style="border-color: var(--border-subtle);">
          <h3 class="text-xs font-semibold text-[var(--text-muted)] uppercase tracking-wider mb-3">Settings</h3>
          <nav class="space-y-0.5">
            {#each [
              { id: 'auth' as SettingsTab, label: 'YouTube認証' },
              { id: 'tts' as SettingsTab, label: 'TTS読み上げ' },
              { id: 'raw' as SettingsTab, label: '生レスポンス保存' },
              { id: 'theme' as SettingsTab, label: 'UIテーマ' }
            ] as item}
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
        <!-- Settings content -->
        <div class="flex-1 overflow-y-auto bg-[var(--bg-base)]">
          <div class="max-w-3xl">
            {#if activeSettingsTab === 'auth'}
              <AuthSettings />
            {:else if activeSettingsTab === 'tts'}
              <TtsSettings />
            {:else if activeSettingsTab === 'raw'}
              <RawResponseSettings />
            {:else if activeSettingsTab === 'theme'}
              {#await import('$lib/components/settings/ThemeSettings.svelte') then module}
                <module.default />
              {/await}
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </main>
</div>

<StorageErrorDialog
  open={showStorageErrorDialog}
  onClose={() => showStorageErrorDialog = false}
  onOpenSettings={openAuthSettings}
/>
