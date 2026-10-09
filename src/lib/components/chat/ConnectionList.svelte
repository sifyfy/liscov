<script lang="ts">
  /**
   * 操作パネルの接続の一覧 (docs/specs/02_chat.md「配信中の画面」の「上の行と操作パネル」)
   *
   * 配信者名・配信タイトル・ジュエル数の注意・切断。2 つ以上なら全切断。省略せず折り返す
   */
  import { chatStore } from '$lib/stores/chat.svelte';

  // 失敗しても一覧からは消える（バックエンド側で既に終わっていた接続など）。未処理の reject にしない
  function handleDisconnect(connectionId: number) {
    chatStore.disconnect(connectionId).catch((e) => console.warn(`切断に失敗 (connection ${connectionId}):`, e));
  }

  function handleDisconnectAll() {
    chatStore.disconnectAll().catch((e) => console.warn('全切断に失敗:', e));
  }
</script>

{#if chatStore.connections.size > 0}
  <section class="flex flex-col gap-1" aria-label="接続">
    <ul class="flex flex-col gap-1">
      {#each [...chatStore.connections.values()] as conn (conn.id)}
        <li data-testid="connection-item" class="flex items-start gap-2 rounded-control bg-surface px-2 py-1">
          <span
            class="mt-1.5 size-1.5 shrink-0 rounded-full {conn.connectionState === 'connected' ? 'bg-success' : 'bg-fg-3'}"
            aria-hidden="true"
          ></span>
          <span class="flex min-w-0 flex-1 flex-col">
            <span class="text-label break-words text-fg-1">{conn.broadcasterName}</span>
            <span class="text-caption break-words text-fg-2" data-testid="stream-title">{conn.streamTitle}</span>
            {#if conn.jewelCountUnavailable}
              <span class="text-caption text-warning" data-testid="jewel-count-notice">
                ジュエル数を取得できません。配信者本人としてログインした接続でのみ取得できます
              </span>
            {/if}
          </span>
          <button
            type="button"
            title="{conn.broadcasterName} を切断"
            onclick={() => handleDisconnect(conn.id)}
            disabled={conn.connectionState !== 'connected'}
            class="shrink-0 rounded-control px-2 py-0.5 text-label whitespace-nowrap text-fg-2 hover:bg-fg-1/8 disabled:opacity-50"
          >
            切断
          </button>
        </li>
      {/each}
    </ul>
    {#if chatStore.connections.size > 1}
      <button
        type="button"
        onclick={handleDisconnectAll}
        class="self-end rounded-control border border-outline px-3 py-0.5 text-label whitespace-nowrap text-fg-2 hover:bg-fg-1/8"
      >
        全切断
      </button>
    {/if}
  </section>
{/if}
