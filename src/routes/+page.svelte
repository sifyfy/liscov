<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { chatStore, configStore, websocketStore, viewerStore, analyticsStore, ttsStore, authStore } from '$lib/stores';
  import { ChatDisplay, FilterPanel, InputSection, StatisticsPanel } from '$lib/components/chat';
  import { ViewerManagement } from '$lib/components/viewer';
  import { RevenueDashboard, ExportPanel } from '$lib/components/analytics';
  import { AuthSettings, TtsSettings, RawResponseSettings } from '$lib/components/settings';
  import { AuthIndicator, StorageErrorDialog } from '$lib/components/auth';

  type Tab = 'chat' | 'viewers' | 'analytics' | 'settings';
  type SettingsTab = 'auth' | 'tts' | 'raw';
  let activeTab = $state<Tab>('chat');
  let activeSettingsTab = $state<SettingsTab>('auth');
  let showStorageErrorDialog = $state(false);

  // Get broadcaster ID from chat connection
  let broadcasterId = $derived(chatStore.broadcasterChannelId || '');

  // Check for storage error
  let hasStorageError = $derived(authStore.storageError !== null);

  onMount(async () => {
    // Load configuration at startup (spec: 09_config.md)
    await configStore.load();
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
    // Check WebSocket status
    await websocketStore.refreshStatus();
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

<div class="flex flex-col h-screen">
  <!-- Header with gradient (original liscov style) -->
  <header class="px-6 py-4" style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <h1 class="text-2xl font-bold text-white">Liscov</h1>
        <span class="text-white/70 text-sm">YouTube Live Chat Monitor</span>
      </div>
      <div class="flex items-center gap-4">
        <!-- Auth indicator -->
        <AuthIndicator onclick={openAuthSettings} />
        <!-- Connection status -->
        <div class="flex items-center gap-2">
          <div
            class="w-2 h-2 rounded-full"
            class:bg-green-400={chatStore.isConnected}
            class:bg-gray-400={!chatStore.isConnected}
          ></div>
          <span class="text-sm text-white/80">
            {chatStore.isConnected ? 'Connected' : 'Disconnected'}
          </span>
        </div>
        <!-- WebSocket status -->
        {#if websocketStore.isRunning}
          <div class="flex items-center gap-2 px-3 py-1 bg-white/20 rounded-lg">
            <div class="w-2 h-2 rounded-full bg-blue-300"></div>
            <span class="text-sm text-white">
              WS: {websocketStore.actualPort}
              ({websocketStore.connectedClients} clients)
            </span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Tab navigation -->
    <div class="flex gap-1 mt-4">
      <button
        onclick={() => (activeTab = 'chat')}
        class="px-4 py-2 rounded-t-lg text-sm font-medium transition-colors {activeTab === 'chat' ? 'bg-white/20 text-white' : 'text-white/70 hover:text-white hover:bg-white/10'}"
      >
        Chat
      </button>
      <button
        onclick={() => (activeTab = 'viewers')}
        class="px-4 py-2 rounded-t-lg text-sm font-medium transition-colors {activeTab === 'viewers' ? 'bg-white/20 text-white' : 'text-white/70 hover:text-white hover:bg-white/10'} {!broadcasterId ? 'opacity-50 cursor-not-allowed' : ''}"
        disabled={!broadcasterId}
      >
        Viewers
        {#if !broadcasterId}
          <span class="text-xs">(connect first)</span>
        {/if}
      </button>
      <button
        onclick={() => (activeTab = 'analytics')}
        class="px-4 py-2 rounded-t-lg text-sm font-medium transition-colors {activeTab === 'analytics' ? 'bg-white/20 text-white' : 'text-white/70 hover:text-white hover:bg-white/10'}"
      >
        Analytics
      </button>
      <button
        onclick={() => (activeTab = 'settings')}
        class="px-4 py-2 rounded-t-lg text-sm font-medium transition-colors {activeTab === 'settings' ? 'bg-white/20 text-white' : 'text-white/70 hover:text-white hover:bg-white/10'}"
      >
        Settings
      </button>
    </div>
  </header>

  <!-- Main content (white/light gray background) -->
  <main class="flex-1 flex overflow-hidden bg-[var(--bg-white)]">
    {#if activeTab === 'chat'}
      <!-- Chat panel -->
      <div class="flex-1 flex flex-col relative bg-[var(--bg-light)]">
        <InputSection />
        <FilterPanel />
        <div class="flex-1 overflow-hidden">
          <ChatDisplay />
        </div>
      </div>

      <!-- Side panel -->
      <aside class="w-80 bg-[var(--bg-white)] border-l border-[var(--border-light)] hidden lg:block overflow-y-auto">
        <div class="p-4 space-y-4">
          <!-- Statistics Panel -->
          <StatisticsPanel />

          <!-- WebSocket controls -->
          <div class="p-4 bg-[var(--bg-white)] rounded-lg border border-[var(--border-light)] shadow-sm">
            <h3 class="text-md font-semibold text-[var(--text-primary)] mb-3">WebSocket API</h3>
            {#if websocketStore.isRunning}
              <div class="space-y-2">
                <p class="text-sm text-[var(--text-secondary)]">
                  Running on port <span class="text-[var(--text-primary)] font-mono">{websocketStore.actualPort}</span>
                </p>
                <p class="text-sm text-[var(--text-secondary)]">
                  <span class="text-[var(--text-primary)]">{websocketStore.connectedClients}</span> clients connected
                </p>
                <button
                  onclick={() => websocketStore.stop()}
                  class="w-full px-4 py-2 bg-red-50 text-red-600 border border-red-200 rounded-lg hover:bg-red-100 transition-colors"
                >
                  Stop Server
                </button>
              </div>
            {:else}
              <button
                onclick={() => websocketStore.start()}
                disabled={websocketStore.isStarting}
                class="w-full px-4 py-2 text-white rounded-lg transition-colors disabled:opacity-50"
                style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);"
              >
                {websocketStore.isStarting ? 'Starting...' : 'Start Server'}
              </button>
            {/if}
            {#if websocketStore.error}
              <p class="mt-2 text-sm text-red-500">{websocketStore.error}</p>
            {/if}
          </div>
        </div>
      </aside>
    {:else if activeTab === 'viewers' && broadcasterId}
      <!-- Viewers panel -->
      <div class="flex-1 p-4 bg-[var(--bg-light)]">
        <ViewerManagement {broadcasterId} />
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
