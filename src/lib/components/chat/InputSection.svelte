<script lang="ts">
  import { chatStore } from '$lib/stores';

  let streamUrl = $state('');

  async function handleConnect() {
    if (!streamUrl.trim()) return;
    await chatStore.connect(streamUrl, chatStore.chatMode);
  }

  async function handlePause() {
    await chatStore.pause();
  }

  async function handleResume() {
    await chatStore.resume();
  }

  async function handleInitialize() {
    streamUrl = '';  // Clear URL first to avoid race condition
    await chatStore.initialize();
  }

  async function toggleChatMode() {
    const newMode = chatStore.chatMode === 'top' ? 'all' : 'top';
    await chatStore.setChatMode(newMode);
  }

  // Statistics
  let messageCount = $derived(chatStore.messages.length);
  let uniqueViewers = $derived(() => {
    const uniqueIds = new Set(chatStore.messages.map(m => m.channel_id));
    return uniqueIds.size;
  });
  let messagesPerMinute = $derived(() => {
    if (chatStore.messages.length < 2) return 0;
    const now = Date.now();
    const oneMinuteAgo = now - 60000;
    const recentMessages = chatStore.messages.filter(m => {
      try {
        const timestamp = parseInt(m.timestamp_usec) / 1000;
        return timestamp > oneMinuteAgo;
      } catch {
        return false;
      }
    });
    return recentMessages.length;
  });
</script>

<!-- Original liscov style: 接続設定 section -->
<div class="p-3 rounded-lg border border-[var(--border-light)]" style="background: linear-gradient(135deg, #f8fafc 0%, #e2e8f0 100%);">
  <!-- Header -->
  <div class="flex items-center gap-2 mb-2">
    <span class="text-base">🔗</span>
    <span class="font-semibold text-[var(--text-primary)]">接続設定</span>
  </div>

  <!-- Input form -->
  {#if chatStore.isConnected}
    <!-- Connected state: Show stream info + 停止 button -->
    <div class="flex items-center gap-2">
      <div class="flex-1 min-w-0 px-3 py-1.5 rounded bg-white border border-[var(--border-light)] truncate text-sm text-[var(--text-secondary)]">
        {chatStore.streamTitle || chatStore.broadcasterName || 'Connected'}
        {#if chatStore.isReplay}
          <span class="ml-1 px-1.5 py-0.5 text-xs bg-blue-100 text-blue-700 rounded">Replay</span>
        {/if}
      </div>
      <button
        onclick={handlePause}
        class="px-3 py-1.5 text-sm bg-orange-500 text-white font-medium rounded hover:bg-orange-600 transition-colors"
      >
        停止
      </button>
      <!-- Chat mode toggle -->
      <button
        onclick={toggleChatMode}
        class="px-3 py-1.5 text-sm rounded font-medium transition-colors {chatStore.chatMode === 'top' ? 'bg-blue-100 text-blue-700 border border-blue-300' : 'bg-orange-100 text-orange-700 border border-orange-300'}"
        title="チャットモード切り替え"
      >
        {chatStore.chatMode === 'top' ? '🔝 トップ' : '💬 全て'}
      </button>
      <!-- Stats -->
      <div class="flex items-center gap-2 ml-2 pl-2 border-l border-[var(--border-light)]">
        <div class="min-w-[4.5rem] px-2 py-1 bg-blue-50 rounded border border-blue-200 text-right">
          <span class="text-sm font-bold text-blue-800">{messageCount}</span>
          <span class="text-xs text-blue-600 ml-1">件</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-green-50 rounded border border-green-200 text-right">
          <span class="text-sm font-bold text-green-800">{messagesPerMinute()}</span>
          <span class="text-xs text-green-600 ml-1">/分</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-yellow-50 rounded border border-yellow-300 text-right">
          <span class="text-sm font-bold text-yellow-800">{uniqueViewers()}</span>
          <span class="text-xs text-yellow-700 ml-1">人</span>
        </div>
      </div>
    </div>
  {:else if chatStore.isPaused}
    <!-- Paused state: Show stream info + 再開/初期化 buttons -->
    <div class="flex items-center gap-2">
      <div class="flex-1 min-w-0 px-3 py-1.5 rounded bg-yellow-50 border border-yellow-300 truncate text-sm text-[var(--text-secondary)]">
        <span class="text-yellow-700 font-medium">⏸ 一時停止中:</span>
        <span class="ml-1">{chatStore.streamTitle || chatStore.broadcasterName || '配信'}</span>
      </div>
      <button
        onclick={handleResume}
        class="px-3 py-1.5 text-sm text-white font-medium rounded transition-colors"
        style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);"
      >
        再開
      </button>
      <button
        onclick={handleInitialize}
        class="px-3 py-1.5 text-sm bg-gray-500 text-white font-medium rounded hover:bg-gray-600 transition-colors"
      >
        初期化
      </button>
      <!-- Chat mode toggle -->
      <button
        onclick={toggleChatMode}
        class="px-3 py-1.5 text-sm rounded font-medium transition-colors {chatStore.chatMode === 'top' ? 'bg-blue-100 text-blue-700 border border-blue-300' : 'bg-orange-100 text-orange-700 border border-orange-300'}"
        title="チャットモード切り替え"
      >
        {chatStore.chatMode === 'top' ? '🔝 トップ' : '💬 全て'}
      </button>
      <!-- Stats (preserved from paused state) -->
      <div class="flex items-center gap-2 ml-2 pl-2 border-l border-[var(--border-light)]">
        <div class="min-w-[4.5rem] px-2 py-1 bg-blue-50 rounded border border-blue-200 text-right">
          <span class="text-sm font-bold text-blue-800">{messageCount}</span>
          <span class="text-xs text-blue-600 ml-1">件</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-green-50 rounded border border-green-200 text-right">
          <span class="text-sm font-bold text-green-800">{messagesPerMinute()}</span>
          <span class="text-xs text-green-600 ml-1">/分</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-yellow-50 rounded border border-yellow-300 text-right">
          <span class="text-sm font-bold text-yellow-800">{uniqueViewers()}</span>
          <span class="text-xs text-yellow-700 ml-1">人</span>
        </div>
      </div>
    </div>
  {:else}
    <div class="flex items-center gap-2">
      <input
        type="text"
        bind:value={streamUrl}
        placeholder="https://www.youtube.com/watch?v=..."
        disabled={chatStore.isConnecting}
        class="flex-1 px-3 py-1.5 text-sm rounded bg-white text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-light)] focus:outline-none focus:ring-2 focus:ring-[var(--primary-start)]/50 disabled:opacity-50"
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            handleConnect();
          }
        }}
      />
      <button
        type="button"
        onclick={handleConnect}
        disabled={chatStore.isConnecting || !streamUrl.trim()}
        class="px-4 py-1.5 text-sm text-white font-medium rounded transition-colors disabled:opacity-50"
        style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);"
      >
        {chatStore.isConnecting ? '接続中...' : '開始'}
      </button>
      <!-- Chat mode toggle -->
      <button
        onclick={toggleChatMode}
        class="px-3 py-1.5 text-sm rounded font-medium transition-colors {chatStore.chatMode === 'top' ? 'bg-blue-100 text-blue-700 border border-blue-300' : 'bg-orange-100 text-orange-700 border border-orange-300'}"
        title="チャットモード切り替え"
      >
        {chatStore.chatMode === 'top' ? '🔝 トップ' : '💬 全て'}
      </button>
      <!-- Stats (faded when not connected) -->
      <div class="flex items-center gap-2 ml-2 pl-2 border-l border-[var(--border-light)] opacity-50">
        <div class="min-w-[4.5rem] px-2 py-1 bg-blue-50 rounded border border-blue-200 text-right">
          <span class="text-sm font-bold text-blue-800">0</span>
          <span class="text-xs text-blue-600 ml-1">件</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-green-50 rounded border border-green-200 text-right">
          <span class="text-sm font-bold text-green-800">0</span>
          <span class="text-xs text-green-600 ml-1">/分</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-yellow-50 rounded border border-yellow-300 text-right">
          <span class="text-sm font-bold text-yellow-800">0</span>
          <span class="text-xs text-yellow-700 ml-1">人</span>
        </div>
      </div>
    </div>
  {/if}

  {#if chatStore.error}
    <p class="mt-2 text-red-500 text-xs">{chatStore.error}</p>
  {/if}
</div>
