<script lang="ts">
  import { chatStore } from '$lib/stores';

  // Calculate statistics from messages
  let messageCount = $derived(chatStore.messages.length);

  // Unique viewers (by channel_id)
  let uniqueViewers = $derived(() => {
    const uniqueIds = new Set(chatStore.messages.map(m => m.channel_id));
    return uniqueIds.size;
  });

  // Questions count (messages containing ? or ？)
  let questionsCount = $derived(() => {
    return chatStore.messages.filter(m =>
      m.content.includes('?') || m.content.includes('？')
    ).length;
  });

  // Engagement rate (unique viewers / total messages)
  let engagementRate = $derived(() => {
    if (messageCount === 0) return 0;
    return (uniqueViewers() / messageCount) * 100;
  });

  // Messages per minute (simple calculation based on recent messages)
  let messagesPerMinute = $derived(() => {
    if (chatStore.messages.length < 2) return 0;

    // Get messages from the last minute
    const now = Date.now();
    const oneMinuteAgo = now - 60000;

    const recentMessages = chatStore.messages.filter(m => {
      // Try to parse timestamp
      try {
        const timestamp = parseInt(m.timestamp_usec) / 1000;
        return timestamp > oneMinuteAgo;
      } catch {
        return false;
      }
    });

    return recentMessages.length;
  });

  // Activity status based on message rate
  let activityStatus = $derived(() => {
    const rate = messagesPerMinute();
    if (rate > 30) return { label: '活発', color: 'text-red-600', icon: '🔥' };
    if (rate > 10) return { label: '普通', color: 'text-orange-500', icon: '📈' };
    if (rate > 0) return { label: '静か', color: 'text-blue-500', icon: '📊' };
    return { label: '休止', color: 'text-gray-400', icon: '💤' };
  });

  // Uptime tracking
  let startTime = $state<number | null>(null);
  let uptime = $state('停止中');

  $effect(() => {
    if (chatStore.isConnected && !startTime) {
      startTime = Date.now();
    } else if (!chatStore.isConnected) {
      startTime = null;
      uptime = '停止中';
    }
  });

  // Update uptime every second
  $effect(() => {
    if (!chatStore.isConnected || !startTime) return;

    const interval = setInterval(() => {
      const seconds = Math.floor((Date.now() - startTime!) / 1000);
      if (seconds < 60) {
        uptime = `${seconds}s`;
      } else if (seconds < 3600) {
        uptime = `${Math.floor(seconds / 60)}m`;
      } else {
        uptime = `${Math.floor(seconds / 3600)}h${Math.floor((seconds % 3600) / 60)}m`;
      }
    }, 1000);

    return () => clearInterval(interval);
  });
</script>

<div class="p-4 bg-[var(--bg-white)] rounded-lg border border-[var(--border-light)] shadow-sm">
  <!-- Header -->
  <div class="flex items-center justify-between mb-3 pb-2 border-b border-[var(--border-light)]">
    <h3 class="text-md font-semibold text-[var(--text-primary)] flex items-center gap-2">
      <span>📊</span>
      統計
    </h3>
    <div class="flex items-center gap-2">
      <!-- Connection status -->
      <div class="flex items-center gap-1">
        <div
          class="w-2 h-2 rounded-full"
          class:bg-green-500={chatStore.isConnected}
          class:bg-gray-400={!chatStore.isConnected}
        ></div>
        <span class="text-xs {chatStore.isConnected ? 'text-green-600' : 'text-gray-500'}">
          {chatStore.isConnected ? '接続中' : '待機中'}
        </span>
      </div>
      <!-- Uptime -->
      <span class="text-xs text-[var(--text-muted)]">{uptime}</span>
      <!-- Live indicator -->
      <span
        class="px-2 py-0.5 text-xs font-bold rounded text-white"
        class:bg-green-500={chatStore.isConnected}
        class:bg-gray-400={!chatStore.isConnected}
      >
        {chatStore.isConnected ? 'LIVE' : 'OFF'}
      </span>
    </div>
  </div>

  <!-- Statistics Grid -->
  <div class="grid grid-cols-5 gap-2">
    <!-- Message Count -->
    <div class="p-2 bg-blue-50 rounded-lg border border-blue-200 text-center">
      <div class="text-lg font-bold text-blue-800">{messageCount}</div>
      <div class="text-xs text-blue-600">メッセージ</div>
    </div>

    <!-- Messages per Minute -->
    <div class="p-2 bg-green-50 rounded-lg border border-green-200 text-center">
      <div class="text-lg font-bold text-green-800">{messagesPerMinute()}</div>
      <div class="text-xs text-green-600">/分</div>
    </div>

    <!-- Unique Viewers -->
    <div class="p-2 bg-yellow-50 rounded-lg border border-yellow-300 text-center">
      <div class="text-lg font-bold text-yellow-800">{uniqueViewers()}</div>
      <div class="text-xs text-yellow-700">視聴者</div>
    </div>

    <!-- Questions -->
    <div class="p-2 bg-pink-50 rounded-lg border border-pink-200 text-center">
      <div class="text-lg font-bold text-pink-700">{questionsCount()}</div>
      <div class="text-xs text-pink-600">質問</div>
    </div>

    <!-- Engagement Rate -->
    <div class="p-2 bg-purple-50 rounded-lg border border-purple-200 text-center">
      <div class="text-sm font-bold text-purple-700">{engagementRate().toFixed(0)}%</div>
      <div class="text-xs text-purple-600">参加度</div>
    </div>
  </div>

  <!-- Stream Info (when connected) -->
  {#if chatStore.isConnected && chatStore.streamTitle}
    <div class="mt-3 pt-2 border-t border-[var(--border-light)]">
      <p class="text-xs text-[var(--text-muted)]">配信中</p>
      <p class="text-sm text-[var(--text-primary)] truncate">{chatStore.streamTitle}</p>
    </div>
  {/if}
</div>
