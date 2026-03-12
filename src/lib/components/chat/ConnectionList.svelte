<script lang="ts">
  import { chatStore } from '$lib/stores/chat.svelte';

  function handleDisconnect(connectionId: number) {
    chatStore.disconnect(connectionId);
  }

  function handleDisconnectAll() {
    chatStore.disconnectAll();
  }
</script>

{#if chatStore.connections.size > 0}
  <div class="connection-list">
    <div class="connection-list-header">
      <span class="connection-count">接続中: {chatStore.connections.size}</span>
      {#if chatStore.connections.size > 1}
        <button class="disconnect-all-btn" onclick={handleDisconnectAll}>全切断</button>
      {/if}
    </div>
    {#each [...chatStore.connections.values()] as conn (conn.id)}
      <div class="connection-item">
        <div class="color-indicator" style="background-color: {conn.color}"></div>
        <div class="connection-info">
          <span class="broadcaster-name">{conn.broadcasterName}</span>
          <span class="stream-title">{conn.streamTitle}</span>
        </div>
        <button
          class="disconnect-btn"
          title="切断"
          onclick={() => handleDisconnect(conn.id)}
          disabled={conn.connectionState === 'disconnecting'}
        >
          ×
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  /* テーマのCSS変数を使用 */
  .connection-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px;
    background: var(--bg-surface-1);
    border-radius: 6px;
  }
  .connection-list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.8em;
    color: var(--text-secondary);
  }
  .disconnect-all-btn {
    font-size: 0.75em;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg-surface-2);
    color: var(--text-secondary);
    border: 1px solid var(--border-color, rgba(255,255,255,0.1));
    cursor: pointer;
  }
  .disconnect-all-btn:hover {
    background: var(--bg-surface-3);
  }
  .connection-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    border-radius: 4px;
    background: var(--bg-surface-2);
  }
  .color-indicator {
    width: 4px;
    height: 24px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .connection-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .broadcaster-name {
    font-size: 0.85em;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .stream-title {
    font-size: 0.75em;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .disconnect-btn {
    padding: 2px 6px;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    border: none;
    cursor: pointer;
    font-size: 1em;
    line-height: 1;
  }
  .disconnect-btn:hover {
    background: rgba(234, 67, 53, 0.2);
    color: #ea4335;
  }
  .disconnect-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
