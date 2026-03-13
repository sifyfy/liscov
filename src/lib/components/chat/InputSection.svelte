<script lang="ts">
  import { chatStore } from '$lib/stores';

  let streamUrl = $state('');

  // 新しい接続を追加（多接続対応：既存の接続はそのまま）
  async function handleConnect() {
    if (!streamUrl.trim()) return;
    await chatStore.connect(streamUrl, chatStore.chatMode);
    // 接続成功後にURLをクリア（次の接続入力の準備）
    if (!chatStore.error) {
      streamUrl = '';
    }
  }

  async function toggleChatMode() {
    const newMode = chatStore.chatMode === 'top' ? 'all' : 'top';
    await chatStore.setChatMode(newMode);
  }

  // 統計情報
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

<!-- 接続設定セクション -->
<div class="p-3 rounded-lg bg-[var(--bg-surface-2)] border border-[var(--border-default)]">
  <!-- URL入力フォーム（常に表示: 多接続のエントリーポイント） -->
  <div class="flex items-center gap-2">
    <span class="flex-shrink-0 font-semibold text-sm text-[var(--text-primary)]">接続設定</span>
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
    <!-- チャットモード切り替え -->
    <button
      onclick={toggleChatMode}
      class="px-3 py-1.5 text-sm rounded font-medium transition-colors {chatStore.chatMode === 'top' ? 'bg-[var(--accent-subtle)] text-[var(--accent)] border border-[var(--border-default)]' : 'bg-[var(--warning-subtle)] text-[var(--warning)] border border-[var(--border-default)]'}"
      title="チャットモード切り替え"
    >
      {chatStore.chatMode === 'top' ? '🔝 トップ' : '💬 全て'}
    </button>
    <!-- 統計（接続中のみ活性表示） -->
    <div class="flex items-center gap-2 ml-2 pl-2 border-l border-[var(--border-default)] {chatStore.isConnected ? '' : 'opacity-50'}">
      <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
        <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">{chatStore.isConnected ? messageCount : 0}</span>
        <span class="text-xs text-[var(--text-secondary)] ml-1">件</span>
      </div>
      <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
        <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">{chatStore.isConnected ? messagesPerMinute() : 0}</span>
        <span class="text-xs text-[var(--text-secondary)] ml-1">/分</span>
      </div>
      <div class="min-w-[4.5rem] px-2 py-1 bg-[var(--bg-surface-3)] rounded border border-[var(--border-default)] text-right">
        <span class="text-sm font-bold text-[var(--text-primary)]" style="font-family: var(--font-mono);">{chatStore.isConnected ? uniqueViewers() : 0}</span>
        <span class="text-xs text-[var(--text-secondary)] ml-1">人</span>
      </div>
    </div>
  </div>

  {#if chatStore.error}
    <p class="mt-2 text-[var(--error)] text-xs">{chatStore.error}</p>
  {/if}

</div>
