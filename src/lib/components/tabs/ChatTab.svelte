<script lang="ts">
  /**
   * チャットのタブ (docs/specs/02_chat.md「配信中の画面」)
   *
   * 上から 操作パネル (開いたときだけ)・流れ・状態の行。
   * 未接続でコメントも無いときは、流れの場所に配信に接続する欄を出す (配信の準備なので操作してよい)
   */
  import { ChatDisplay, ConnectForm, ControlPanel, StatusLine } from '$lib/components/chat';
  import { chatStore } from '$lib/stores';
  import ErrorBoundary from '$lib/components/ui/ErrorBoundary.svelte';
  import { countRecentMessages, shownCountLabel } from '$lib/utils/chat-stats';
  import { emptyReactions, reactionsLabel, statusReactions } from '$lib/utils/reactions';

  interface Props {
    panelOpen: boolean;
    /** 接続が成立したとき (接続したら操作パネルを隠す) */
    onConnected: () => void;
    onOpenAuthSettings: () => void;
  }

  let { panelOpen, onConnected, onOpenAuthSettings }: Props = $props();

  // 成立した接続も、表示するコメントも無い (接続の途中は欄を出したまま。失敗したら入力をそのまま直せる)
  let showConnectInFlow = $derived(
    chatStore.messages.length === 0 &&
      [...chatStore.connections.values()].every((conn) => conn.connectionState === 'connecting')
  );

  // 状態の行の統計 (02_chat.md「状態の行の統計（StatusLine）」)。
  // コメントが止まっても /分 が下がっていくよう、いまの時刻を 5 秒ごとに進める
  const STATS_TICK_MS = 5000;
  let nowMs = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (nowMs = Date.now()), STATS_TICK_MS);
    return () => clearInterval(timer);
  });
  let perMinute = $derived(countRecentMessages(chatStore.messages, nowMs));
  let shownLabel = $derived(shownCountLabel(chatStore.displayedMessages.length, chatStore.messages.length));
  // リアクションは主の配信のものだけ (02_chat.md「状態の行のリアクション」)
  let primaryReactions = $derived(chatStore.primaryConnection?.reactions ?? emptyReactions);
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-surface">
  {#if panelOpen}
    <ControlPanel showConnect={!showConnectInFlow} {onConnected} {onOpenAuthSettings} />
  {/if}

  <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
    {#if showConnectInFlow}
      <div class="flex flex-1 items-center justify-center p-4">
        <div class="w-full max-w-md">
          <ConnectForm {onConnected} />
        </div>
      </div>
    {:else}
      <ErrorBoundary name="チャット一覧">
        <ChatDisplay />
      </ErrorBoundary>
    {/if}
  </div>

  <StatusLine
    connected={chatStore.isConnected}
    count={chatStore.messages.length}
    {perMinute}
    viewers={chatStore.viewerCount}
    {shownLabel}
    reactions={statusReactions(primaryReactions)}
    reactionsLabel={reactionsLabel(primaryReactions)}
    error={chatStore.error}
    onDismissError={() => chatStore.dismissError()}
  />
</div>
