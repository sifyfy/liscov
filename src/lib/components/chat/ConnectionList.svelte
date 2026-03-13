<script lang="ts">
  import { chatStore } from '$lib/stores/chat.svelte';

  function handleDisconnect(connectionId: number) {
    chatStore.disconnect(connectionId);
  }

</script>

{#if chatStore.connections.size > 0}
  <div class="connection-list">
    {#each [...chatStore.connections.values()] as conn (conn.id)}
      <div class="connection-item">
        <div class="color-indicator" style="background-color: {conn.color}"></div>
        <div class="connection-info">
          <span class="broadcaster-name">{conn.broadcasterName}</span>
          <span class="stream-title" data-testid="stream-title">{conn.streamTitle}</span>
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
    gap: 2px;
    padding: 0 8px 4px;
    background: var(--bg-surface-1);
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
