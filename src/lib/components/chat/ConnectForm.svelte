<script lang="ts">
  /**
   * 配信に接続する欄 (docs/specs/02_chat.md「配信中の画面」の「上の行と操作パネル」)
   *
   * 配信の URL・すべてのコメントを取るか・「接続」のボタン。操作パネルの中と、未接続でコメントも無いときの流れの場所で使う。
   * 「すべてのコメントを取る」はチャットモードの「全て」で、接続中のすべての配信とこの後の接続に効く
   */
  import { chatStore } from '$lib/stores';

  interface Props {
    /** 接続が成立したとき (接続したら操作パネルを隠すため) */
    onConnected?: () => void;
  }

  let { onConnected }: Props = $props();

  let streamUrl = $state('');

  async function handleConnect() {
    const url = streamUrl;
    if (!url.trim()) return;
    await chatStore.connect(url, chatStore.chatMode);
    if (chatStore.error) return;
    // 次の接続の入力に備えて空にする。待つ間に入力し直されていたら残す (02_chat.md「多接続」)
    if (streamUrl === url) streamUrl = '';
    onConnected?.();
  }
</script>

<div class="flex w-full flex-col gap-2">
  <label for="connect-url" class="text-label text-fg-1">配信の URL</label>
  <div class="flex gap-2">
    <input
      id="connect-url"
      type="text"
      bind:value={streamUrl}
      placeholder="https://www.youtube.com/watch?v=..."
      disabled={chatStore.isConnecting}
      onkeydown={(e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          handleConnect();
        }
      }}
      class="min-w-0 flex-1 rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 disabled:opacity-50"
    />
    <button
      type="button"
      onclick={handleConnect}
      disabled={chatStore.isConnecting || !streamUrl.trim()}
      class="shrink-0 rounded-control bg-accent px-4 py-1.5 text-label whitespace-nowrap text-surface disabled:opacity-50"
    >
      {chatStore.isConnecting ? '接続中…' : '接続'}
    </button>
  </div>
  <label class="flex items-center gap-1.5 text-label text-fg-2">
    <input
      type="checkbox"
      checked={chatStore.chatMode === 'all'}
      onchange={(e) => chatStore.setChatMode(e.currentTarget.checked ? 'all' : 'top')}
      class="accent-accent"
    />
    すべてのコメントを取る (トップのチャットにしない)
  </label>
</div>
