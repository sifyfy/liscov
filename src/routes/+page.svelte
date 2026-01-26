<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { chatStore, configStore, websocketStore, viewerStore, analyticsStore, ttsStore, authStore } from '$lib/stores';
  import { ChatDisplay, FilterPanel, InputSection } from '$lib/components/chat';
  import { ViewerManagement } from '$lib/components/viewer';
  import { RevenueDashboard, ExportPanel } from '$lib/components/analytics';
  import { AuthSettings, TtsSettings, RawResponseSettings } from '$lib/components/settings';
  import { AuthIndicator, StorageErrorDialog } from '$lib/components/auth';

  type Tab = 'chat' | 'viewers' | 'analytics' | 'settings';
  type SettingsTab = 'auth' | 'tts' | 'raw';
  let activeTab = $state<Tab>('chat');
  let activeSettingsTab = $state<SettingsTab>('auth');
  let showStorageErrorDialog = $state(false);

  // Tab metadata for integrated header (original liscov style)
  const tabInfo: Record<Tab, { icon: string; label: string; description: string }> = {
    chat: { icon: '🔍', label: 'Chat Monitor', description: 'リアルタイムチャット監視' },
    viewers: { icon: '👥', label: 'Viewer Management', description: '視聴者情報管理' },
    analytics: { icon: '📊', label: 'Revenue Analytics', description: '収益分析・統計' },
    settings: { icon: '⚙️', label: 'Settings', description: 'アプリケーション設定' }
  };

  // Current tab info derived
  let currentTab = $derived(tabInfo[activeTab]);

  // Get broadcaster ID from chat connection
  let broadcasterId = $derived(chatStore.broadcasterChannelId || '');

  // Check for storage error
  let hasStorageError = $derived(authStore.storageError !== null);

  onMount(async () => {
    // Load configuration at startup (spec: 09_config.md)
    await configStore.load();
    // Initialize chat display settings from config (spec: 09_config.md)
    chatStore.initDisplaySettings();
    // Check auth status (spec: 01_auth.md)
    await authStore.refreshStatus();
    // Show storage error dialog if needed
    if (authStore.storageError) {
      showStorageErrorDialog = true;
    }
    // Check session validity if authenticated
    if (authStore.isAuthenticated) {
      authStore.checkSessionValidity();
    }
    // Setup event listeners for Tauri events
    await chatStore.setupEventListeners();
    // Initialize WebSocket store (sets up event listeners and fetches status)
    await websocketStore.init();
  });

  function openAuthSettings() {
    activeTab = 'settings';
    activeSettingsTab = 'auth';
  }

  onDestroy(() => {
    // Cleanup event listeners
    chatStore.cleanup();
  });
</script>

<!-- Main window with gradient background (original liscov style) -->
<div class="h-screen flex flex-col p-1 overflow-hidden" style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);">
  <!-- Integrated Header & Tab Navigation (original liscov style) -->
  <header
    class="flex items-center justify-between mb-1.5 flex-shrink-0"
    style="
      background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);
      border-radius: 12px;
      padding: 6px 12px;
      box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);
      backdrop-filter: blur(10px);
      border: 1px solid rgba(255, 255, 255, 0.2);
      min-height: 52px;
    "
  >
    <!-- Left: Active tab info -->
    <div class="flex items-center gap-3 flex-1 min-w-0">
      <!-- Tab icon (large) -->
      <div
        class="flex items-center justify-center text-2xl"
        style="
          width: 48px;
          height: 48px;
          background: rgba(255, 255, 255, 0.15);
          border-radius: 12px;
          backdrop-filter: blur(10px);
          border: 1px solid rgba(255, 255, 255, 0.2);
          box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
        "
      >
        {currentTab.icon}
      </div>

      <!-- Tab info text -->
      <div class="flex flex-col gap-0.5 flex-1 min-w-0">
        <h1 class="text-lg font-bold text-white leading-tight" style="text-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);">
          {currentTab.label}
        </h1>
        <p class="text-sm text-white/80 leading-tight truncate" style="text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);">
          {currentTab.description}
        </p>
      </div>

      <!-- Status indicators (compact) -->
      <div class="flex items-center gap-2 flex-shrink-0">
        <AuthIndicator onclick={openAuthSettings} />
        <div class="flex items-center gap-1.5 px-2 py-1 rounded-lg" style="background: rgba(255, 255, 255, 0.1);">
          <div
            class="w-2 h-2 rounded-full"
            class:bg-green-400={chatStore.isConnected}
            class:bg-gray-400={!chatStore.isConnected}
          ></div>
          <span class="text-xs text-white/80">
            {chatStore.isConnected ? '接続中' : '未接続'}
          </span>
        </div>
        {#if websocketStore.isRunning}
          <div class="flex items-center gap-1.5 px-2 py-1 rounded-lg" style="background: rgba(255, 255, 255, 0.1);">
            <div class="w-2 h-2 rounded-full bg-blue-300"></div>
            <span class="text-xs text-white">
              WS:{websocketStore.actualPort}({websocketStore.connectedClients})
            </span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Right: Tab navigation (glass morphism container) -->
    <nav
      class="flex gap-1 flex-shrink-0 ml-4"
      style="
        background: rgba(255, 255, 255, 0.1);
        border-radius: 10px;
        padding: 4px;
        backdrop-filter: blur(10px);
        border: 1px solid rgba(255, 255, 255, 0.15);
      "
    >
      {#each (['chat', 'viewers', 'analytics', 'settings'] as const) as tab}
        <button
          onclick={() => (activeTab = tab)}
          class="flex items-center justify-center gap-1 transition-all"
          style={activeTab === tab
            ? 'background: rgba(255, 255, 255, 0.95); color: #333; font-weight: 700; padding: 6px 10px; border-radius: 7px; box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15); transform: translateY(-1px); min-width: 70px; font-size: 11px;'
            : 'background: transparent; color: rgba(255, 255, 255, 0.7); font-weight: 500; padding: 6px 10px; border-radius: 7px; min-width: 70px; font-size: 11px;'}
        >
          <span class="text-xs">{tabInfo[tab].icon}</span>
          <span class="truncate max-w-[50px] text-[10px]">{tabInfo[tab].label.split(' ')[0]}</span>
        </button>
      {/each}
    </nav>
  </header>

  <!-- Main content area (original liscov layout) -->
  <main class="flex-1 flex flex-col overflow-hidden rounded-lg bg-[var(--bg-white)]">
    {#if activeTab === 'chat'}
      <!-- Top section: Input with integrated stats -->
      <div class="p-3 bg-[var(--bg-white)]">
        <InputSection />
      </div>

      <!-- Control bar: Filter, auto-scroll, timestamp, etc. -->
      <FilterPanel />

      <!-- Chat display area (fills remaining space) -->
      <div class="flex-1 overflow-hidden">
        <ChatDisplay />
      </div>
    {:else if activeTab === 'viewers'}
      <!-- Viewers panel -->
      <div class="flex-1 p-4 bg-[var(--bg-light)]">
        <ViewerManagement broadcasterId={broadcasterId || undefined} />
      </div>
    {:else if activeTab === 'analytics'}
      <!-- Analytics panel -->
      <div class="flex-1 p-6 overflow-y-auto bg-[var(--bg-light)]">
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
      <!-- Settings panel -->
      <div class="flex-1 flex overflow-hidden bg-[var(--bg-light)]">
        <!-- Settings sidebar -->
        <div class="w-48 bg-[var(--bg-white)] border-r border-[var(--border-light)] p-4">
          <h3 class="text-sm font-semibold text-[var(--text-muted)] mb-3">設定</h3>
          <nav class="space-y-1">
            <button
              onclick={() => (activeSettingsTab = 'auth')}
              class="w-full text-left px-3 py-2 rounded-lg text-sm transition-colors {activeSettingsTab === 'auth' ? 'bg-[var(--primary-start)]/10 text-[var(--primary-start)] font-medium' : 'text-[var(--text-secondary)] hover:bg-[var(--bg-light)] hover:text-[var(--text-primary)]'}"
            >
              YouTube認証
            </button>
            <button
              onclick={() => (activeSettingsTab = 'tts')}
              class="w-full text-left px-3 py-2 rounded-lg text-sm transition-colors {activeSettingsTab === 'tts' ? 'bg-[var(--primary-start)]/10 text-[var(--primary-start)] font-medium' : 'text-[var(--text-secondary)] hover:bg-[var(--bg-light)] hover:text-[var(--text-primary)]'}"
            >
              TTS読み上げ
            </button>
            <button
              onclick={() => (activeSettingsTab = 'raw')}
              class="w-full text-left px-3 py-2 rounded-lg text-sm transition-colors {activeSettingsTab === 'raw' ? 'bg-[var(--primary-start)]/10 text-[var(--primary-start)] font-medium' : 'text-[var(--text-secondary)] hover:bg-[var(--bg-light)] hover:text-[var(--text-primary)]'}"
            >
              生レスポンス保存
            </button>
          </nav>
        </div>
        <!-- Settings content -->
        <div class="flex-1 overflow-y-auto bg-[var(--bg-light)]">
          <div class="max-w-3xl">
            {#if activeSettingsTab === 'auth'}
              <AuthSettings />
            {:else if activeSettingsTab === 'tts'}
              <TtsSettings />
            {:else if activeSettingsTab === 'raw'}
              <RawResponseSettings />
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </main>
</div>

<!-- Storage Error Dialog -->
<StorageErrorDialog
  open={showStorageErrorDialog}
  onClose={() => showStorageErrorDialog = false}
  onOpenSettings={openAuthSettings}
/>
