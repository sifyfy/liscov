<script lang="ts">
  import { tick } from 'svelte';
  import { chatStore, authStore, configStore } from '$lib/stores';
  import { StorageErrorDialog } from '$lib/components/auth';
  import { ViewerManagement } from '$lib/components/viewer';
  import { ChatTab, AnalyticsTab, SettingsTab } from '$lib/components/tabs';
  import ErrorBoundary from '$lib/components/ui/ErrorBoundary.svelte';
  import TopBar, { type AppTab } from './TopBar.svelte';

  type SettingsSubTab = 'auth' | 'tts' | 'raw' | 'theme';

  // アクティブタブと設定サブタブの状態
  let activeTab = $state<AppTab>('chat');
  let activeSettingsSubTab = $state<SettingsSubTab>('auth');
  let showStorageErrorDialog = $state(false);
  // 操作パネル (チャットのタブにだけある。02_chat.md「上の行と操作パネル」)
  let panelOpen = $state(false);

  // 最初の接続のbroadcasterChannelIdを使用（視聴者タブの既定の配信者）
  let broadcasterId = $derived(chatStore.broadcasterChannelId ?? '');

  // ストレージエラーが発生した場合にダイアログを表示
  let hasStorageError = $derived(authStore.storageError !== null);
  $effect(() => {
    if (hasStorageError) {
      showStorageErrorDialog = true;
    }
  });

  // 認証設定画面を開く（AuthIndicator・StorageErrorDialog からのコールバック用）
  function openAuthSettings() {
    activeTab = 'settings';
    activeSettingsSubTab = 'auth';
  }

  // チャット以外のタブで押したら、チャットのタブに移って開く
  function togglePanel() {
    if (activeTab !== 'chat') {
      activeTab = 'chat';
      panelOpen = true;
    } else {
      panelOpen = !panelOpen;
    }
  }

  // 「+」: 操作パネルを開いて配信の URL の入力に移る (未接続なら流れの場所の入力)
  async function addConnection() {
    activeTab = 'chat';
    panelOpen = true;
    await tick();
    document.getElementById('connect-url')?.focus();
  }

  // 接続したら操作パネルを隠す (設定で無効にできる)
  function handleConnected() {
    if (configStore.hidePanelOnConnect) panelOpen = false;
  }

  function disconnect(connectionId: number) {
    chatStore.disconnect(connectionId).catch((e) => console.warn(`切断に失敗 (connection ${connectionId}):`, e));
  }
</script>

<div class="flex h-screen flex-col overflow-hidden bg-surface font-ui text-fg-1">
  <TopBar
    {activeTab}
    onSelectTab={(tab) => (activeTab = tab)}
    connections={[...chatStore.connections.values()]}
    primaryId={chatStore.primaryConnection?.id}
    {panelOpen}
    onTogglePanel={togglePanel}
    onAddConnection={addConnection}
    onDisconnect={disconnect}
  />

  <!-- メインコンテンツエリア -->
  <main class="flex min-h-0 flex-1 flex-col overflow-hidden">
    <!-- Chatタブ: VList の再マウントコストを避けるため display:none で維持 -->
    <div style:display={activeTab === 'chat' ? 'flex' : 'none'} class="flex min-h-0 flex-1 flex-col overflow-hidden">
      <ErrorBoundary name="チャットタブ">
        <ChatTab {panelOpen} onConnected={handleConnected} onOpenAuthSettings={openAuthSettings} />
      </ErrorBoundary>
    </div>
    {#if activeTab === 'viewers'}
      <div class="flex-1 overflow-y-auto bg-surface p-4">
        <ErrorBoundary name="視聴者タブ">
          <ViewerManagement broadcasterId={broadcasterId || undefined} />
        </ErrorBoundary>
      </div>
    {:else if activeTab === 'analytics'}
      <ErrorBoundary name="分析タブ">
        <AnalyticsTab />
      </ErrorBoundary>
    {:else if activeTab === 'settings'}
      <ErrorBoundary name="設定タブ">
        <SettingsTab initialTab={activeSettingsSubTab} />
      </ErrorBoundary>
    {/if}
  </main>
</div>

<!-- ストレージエラーダイアログ -->
<StorageErrorDialog
  open={showStorageErrorDialog}
  onClose={() => (showStorageErrorDialog = false)}
  onOpenSettings={openAuthSettings}
/>
