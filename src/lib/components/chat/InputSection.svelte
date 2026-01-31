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
<div class="p-3 rounded-lg bg-[var(--bg-surface-2)] border border-[var(--border-default)]">
  <!-- Header -->
  <div class="flex items-center gap-2 mb-2">
    <span class="font-semibold text-[var(--text-primary)]">接続設定</span>
  </div>

  <!-- Input form -->
  {#if chatStore.isConnected}
    <!-- Connected state: Show stream info + 停止 button -->
    <div class="flex items-center gap-2">
      <div class="flex-1 min-w-0 px-3 py-1.5 rounded bg-[var(--bg-surface-2)] border border-[var(--border-default)] truncate text-sm text-[var(--text-secondary)]">
        {chatStore.streamTitle || chatStore.broadcasterName || 'Connected'}
        {#if chatStore.isReplay}
          <span class="ml-1 px-1.5 py-0.5 text-xs bg-[var(--accent-subtle)] text-[var(--accent)] rounded">Replay</span>
        {/if}
      </div>
      <button
        onclick={handlePause}
        class="px-3 py-1.5 text-sm text-[var(--text-inverse)] font-medium rounded transition-colors"
        style="background: var(--warning);"
      >
        停止
      </button>
      <!-- Chat mode toggle -->
      <button
        onclick={toggleChatMode}
        class="px-3 py-1.5 text-sm rounded font-medium transition-colors {chatStore.chatMode === 'top' ? 'bg-[var(--accent-subtle)] text-[var(--accent)] border border-[var(--border-default)]' : 'bg-[var(--warning-subtle)] text-[var(--warning)] border border-[var(--border-default)]'}"
        title="チャットモード切り替え"
      >
        {chatStore.chatMode === 'top' ? '🔝 トップ' : '💬 全て'}
      </button>
      <!-- Stats -->
      <div class="flex items-center gap-2 ml-2 pl-2 border-l border-[var(--border-default)]">
        <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
          <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">{messageCount}</span>
          <span class="text-xs text-[var(--text-secondary)] ml-1">件</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
          <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">{messagesPerMinute()}</span>
          <span class="text-xs text-[var(--text-secondary)] ml-1">/分</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
          <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">{uniqueViewers()}</span>
          <span class="text-xs text-[var(--text-secondary)] ml-1">人</span>
        </div>
      </div>
    </div>
  {:else if chatStore.isPaused}
    <!-- Paused state: Show stream info + 再開/初期化 buttons -->
    <div class="flex items-center gap-2">
      <div class="flex-1 min-w-0 px-3 py-1.5 rounded bg-[var(--warning-subtle)] border border-[var(--border-default)] truncate text-sm text-[var(--text-secondary)]">
        <span class="text-[var(--warning)] font-medium">⏸ 一時停止中:</span>
        <span class="ml-1">{chatStore.streamTitle || chatStore.broadcasterName || '配信'}</span>
      </div>
      <button
        onclick={handleResume}
        class="px-3 py-1.5 text-sm text-[var(--text-inverse)] font-medium rounded transition-colors"
        style="background: var(--accent);"
      >
        再開
      </button>
      <button
        onclick={handleInitialize}
        class="px-3 py-1.5 text-sm bg-[var(--bg-surface-3)] text-[var(--text-primary)] font-medium rounded transition-colors"
      >
        初期化
      </button>
      <!-- Chat mode toggle -->
      <button
        onclick={toggleChatMode}
        class="px-3 py-1.5 text-sm rounded font-medium transition-colors {chatStore.chatMode === 'top' ? 'bg-[var(--accent-subtle)] text-[var(--accent)] border border-[var(--border-default)]' : 'bg-[var(--warning-subtle)] text-[var(--warning)] border border-[var(--border-default)]'}"
        title="チャットモード切り替え"
      >
        {chatStore.chatMode === 'top' ? '🔝 トップ' : '💬 全て'}
      </button>
      <!-- Stats (preserved from paused state) -->
      <div class="flex items-center gap-2 ml-2 pl-2 border-l border-[var(--border-default)]">
        <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
          <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">{messageCount}</span>
          <span class="text-xs text-[var(--text-secondary)] ml-1">件</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
          <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">{messagesPerMinute()}</span>
          <span class="text-xs text-[var(--text-secondary)] ml-1">/分</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
          <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">{uniqueViewers()}</span>
          <span class="text-xs text-[var(--text-secondary)] ml-1">人</span>
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
        class="flex-1 px-3 py-1.5 text-sm rounded bg-[var(--bg-surface-2)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50 disabled:opacity-50"
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
        class="px-4 py-1.5 text-sm text-[var(--text-inverse)] font-medium rounded transition-colors disabled:opacity-50"
        style="background: var(--accent);"
      >
        {chatStore.isConnecting ? '接続中...' : '開始'}
      </button>
      <!-- Chat mode toggle -->
      <button
        onclick={toggleChatMode}
        class="px-3 py-1.5 text-sm rounded font-medium transition-colors {chatStore.chatMode === 'top' ? 'bg-[var(--accent-subtle)] text-[var(--accent)] border border-[var(--border-default)]' : 'bg-[var(--warning-subtle)] text-[var(--warning)] border border-[var(--border-default)]'}"
        title="チャットモード切り替え"
      >
        {chatStore.chatMode === 'top' ? '🔝 トップ' : '💬 全て'}
      </button>
      <!-- Stats (faded when not connected) -->
      <div class="flex items-center gap-2 ml-2 pl-2 border-l border-[var(--border-default)] opacity-50">
        <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
          <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">0</span>
          <span class="text-xs text-[var(--text-secondary)] ml-1">件</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
          <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">0</span>
          <span class="text-xs text-[var(--text-secondary)] ml-1">/分</span>
        </div>
        <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
          <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">0</span>
          <span class="text-xs text-[var(--text-secondary)] ml-1">人</span>
        </div>
      </div>
    </div>
  {/if}

  {#if chatStore.error}
    <p class="mt-2 text-[var(--error)] text-xs">{chatStore.error}</p>
  {/if}
</div>
