<script lang="ts">
  /**
   * 操作パネル (docs/specs/02_chat.md「配信中の画面」の「上の行と操作パネル」)
   *
   * 準備・合間・配信後の操作をまとめる。配信中は閉じておける (配信中は操作ゼロが前提)。
   * 接続の一覧・配信に接続・表示の設定・ログインと WebSocket の状態・「接続したら操作パネルを隠す」
   */
  import { authStore, chatStore, configStore, websocketStore } from '$lib/stores';
  import { AuthIndicator } from '$lib/components/auth';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ConnectionList from './ConnectionList.svelte';
  import ConnectForm from './ConnectForm.svelte';

  interface Props {
    /** 配信に接続する欄を出すか (未接続でコメントも無いときは流れの場所に出すので、ここには出さない) */
    showConnect: boolean;
    onConnected: () => void;
    onOpenAuthSettings: () => void;
  }

  let { showConnect, onConnected, onOpenAuthSettings }: Props = $props();

  let showClearConfirm = $state(false);

  function confirmClear() {
    chatStore.clearMessages();
    showClearConfirm = false;
  }
</script>

<section
  aria-label="操作パネル"
  class="flex max-h-1/2 shrink-0 flex-col gap-3 overflow-y-auto border-b border-outline-variant bg-surface-low px-3 py-2"
>
  <ConnectionList />

  {#if showConnect}
    <ConnectForm {onConnected} />
  {/if}

  <!-- 表示の設定 -->
  <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-label text-fg-1">
    <span class="flex items-center gap-1">
      文字
      <button
        type="button"
        aria-label="文字を小さく"
        title="文字を小さく"
        onclick={() => chatStore.decreaseFontSize()}
        class="flex size-7 items-center justify-center rounded-control border border-outline hover:bg-fg-1/8"
      >
        <Icon name="minus" size={14} />
      </button>
      <span class="w-10 text-center text-caption text-fg-2 tabular-nums">{chatStore.messageFontSize}px</span>
      <button
        type="button"
        aria-label="文字を大きく"
        title="文字を大きく"
        onclick={() => chatStore.increaseFontSize()}
        class="flex size-7 items-center justify-center rounded-control border border-outline hover:bg-fg-1/8"
      >
        <Icon name="plus" size={14} />
      </button>
    </span>
    <label class="flex items-center gap-1.5 whitespace-nowrap">
      <input
        type="checkbox"
        checked={chatStore.showTimestamps}
        onchange={(e) => chatStore.setShowTimestamps(e.currentTarget.checked)}
        class="accent-accent"
      />
      時刻
    </label>
    <label class="flex items-center gap-1.5 whitespace-nowrap">
      表示件数
      <!-- 選択肢の value は文字列。数値のまま渡すと一致せず空欄になる -->
      <select
        value={chatStore.displayLimit ? String(chatStore.displayLimit) : 'unlimited'}
        onchange={(e) =>
          chatStore.setDisplayLimit(e.currentTarget.value === 'unlimited' ? null : parseInt(e.currentTarget.value))}
        class="rounded-control border border-outline bg-surface px-1.5 py-1 text-label text-fg-1"
      >
        <option value="unlimited">無制限</option>
        <option value="500">500件</option>
      </select>
    </label>
    <span class="flex flex-wrap items-center gap-x-3 gap-y-1">
      <label class="flex items-center gap-1 whitespace-nowrap">
        <input
          type="checkbox"
          checked={chatStore.filter.showText}
          onchange={(e) => chatStore.setFilter({ showText: e.currentTarget.checked })}
          class="accent-accent"
        />
        通常
      </label>
      <label class="flex items-center gap-1 whitespace-nowrap">
        <input
          type="checkbox"
          checked={chatStore.filter.showSuperchat}
          onchange={(e) => chatStore.setFilter({ showSuperchat: e.currentTarget.checked })}
          class="accent-accent"
        />
        スパチャ等
      </label>
      <label class="flex items-center gap-1 whitespace-nowrap">
        <input
          type="checkbox"
          checked={chatStore.filter.showMembership}
          onchange={(e) => chatStore.setFilter({ showMembership: e.currentTarget.checked })}
          class="accent-accent"
        />
        メンバー
      </label>
    </span>
    <input
      type="search"
      aria-label="コメントを探す"
      placeholder="コメントを探す"
      value={chatStore.filter.searchQuery}
      oninput={(e) => chatStore.setFilter({ searchQuery: e.currentTarget.value })}
      class="min-w-40 flex-1 rounded-control border border-outline bg-surface px-2 py-1 text-label text-fg-1 placeholder:text-fg-3"
    />
    <button
      type="button"
      onclick={() => (showClearConfirm = true)}
      disabled={chatStore.messages.length === 0}
      class="rounded-control border border-outline px-3 py-1 text-label whitespace-nowrap text-error hover:bg-fg-1/8 disabled:opacity-50"
    >
      コメントを消す
    </button>
  </div>

  <!-- ログイン・WebSocket・操作パネルの設定 -->
  <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-caption text-fg-2">
    <AuthIndicator state={authStore.indicatorState} onclick={onOpenAuthSettings} />
    <span data-testid="websocket-status" class="flex items-center gap-1.5 whitespace-nowrap">
      <span class="size-1.5 rounded-full {websocketStore.isRunning ? 'bg-success' : 'bg-fg-3'}" aria-hidden="true"></span>
      {#if websocketStore.isRunning}
        WebSocket: ポート {websocketStore.actualPort}・{websocketStore.connectedClients} 件の接続
      {:else}
        WebSocket: 止まっています
      {/if}
    </span>
    {#if websocketStore.error}
      <span class="text-error">{websocketStore.error}</span>
    {/if}
    <label class="ml-auto flex items-center gap-1.5 whitespace-nowrap">
      <input
        type="checkbox"
        checked={configStore.hidePanelOnConnect}
        onchange={(e) => configStore.setHidePanelOnConnect(e.currentTarget.checked)}
        class="accent-accent"
      />
      接続したら操作パネルを隠す
    </label>
  </div>
</section>

{#if showClearConfirm}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-surface/80">
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="clear-title"
      class="mx-4 max-w-sm rounded-popup bg-surface-high p-4 shadow-elevation-2"
    >
      <h3 id="clear-title" class="mb-2 text-title text-fg-1">コメントを消す</h3>
      <p class="mb-4 text-body text-fg-2">
        {chatStore.messages.length}件のコメントをすべて消しますか？<br />この操作は取り消せません。
      </p>
      <div class="flex justify-end gap-2">
        <button
          type="button"
          onclick={() => (showClearConfirm = false)}
          class="rounded-control border border-outline px-4 py-1.5 text-label text-fg-2 hover:bg-fg-1/8"
        >
          キャンセル
        </button>
        <button
          type="button"
          onclick={confirmClear}
          class="rounded-control bg-error px-4 py-1.5 text-label text-surface hover:opacity-90"
        >
          消す
        </button>
      </div>
    </div>
  </div>
{/if}
