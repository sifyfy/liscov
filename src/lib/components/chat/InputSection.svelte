<script lang="ts">
  import { chatStore } from '$lib/stores';

  let streamUrl = $state('');

  async function handleConnect() {
    if (!streamUrl.trim()) return;
    await chatStore.connect(streamUrl, chatStore.chatMode);
  }

  async function handleDisconnect() {
    await chatStore.disconnect();
  }
</script>

<div class="p-4 bg-[var(--bg-white)] border-b border-[var(--border-light)]">
  {#if chatStore.isConnected}
    <!-- Connected state -->
    <div class="flex items-center justify-between">
      <div class="flex-1 min-w-0">
        <p class="text-[var(--text-primary)] font-semibold truncate">
          {chatStore.streamTitle || 'Connected'}
        </p>
        {#if chatStore.broadcasterName}
          <p class="text-[var(--text-secondary)] text-sm truncate">
            {chatStore.broadcasterName}
            {#if chatStore.isReplay}
              <span class="ml-2 px-2 py-0.5 text-xs bg-blue-100 text-blue-700 rounded border border-blue-200">Replay</span>
            {/if}
          </p>
        {/if}
      </div>
      <button
        onclick={handleDisconnect}
        class="px-4 py-2 bg-red-500 text-white font-semibold rounded-lg hover:bg-red-600 transition-colors"
      >
        Disconnect
      </button>
    </div>
  {:else}
    <!-- Disconnected state -->
    <form
      class="flex gap-4"
      onsubmit={(e) => {
        e.preventDefault();
        handleConnect();
      }}
    >
      <input
        type="text"
        bind:value={streamUrl}
        placeholder="Enter YouTube URL or Video ID..."
        disabled={chatStore.isConnecting}
        class="flex-1 px-4 py-2 rounded-lg bg-[var(--bg-light)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-light)] focus:outline-none focus:ring-2 focus:ring-[var(--primary-start)] disabled:opacity-50"
      />
      <button
        type="submit"
        disabled={chatStore.isConnecting || !streamUrl.trim()}
        class="px-6 py-2 text-white font-semibold rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);"
      >
        {#if chatStore.isConnecting}
          Connecting...
        {:else}
          Connect
        {/if}
      </button>
    </form>

    {#if chatStore.error}
      <p class="mt-2 text-red-500 text-sm">{chatStore.error}</p>
    {/if}
  {/if}
</div>
