<script lang="ts">
  /**
   * 状態の行 (docs/specs/02_chat.md「配信中の画面」の「状態」と「状態の行の統計（StatusLine）」)
   *
   * 下端の 1 行。ふだんは控えめに、異常 (エラー) のときだけ error の面にして理由を全文で出す。
   * 値は呼び出し側 (ChatTab) が store から求めて渡す
   */
  import Icon from '$lib/components/ui/Icon.svelte';
  import { formatCount } from '$lib/utils/format';
  import { STATUS_ALERT_LABELS, type StatusAlert } from '$lib/utils/status-alerts';

  interface Props {
    /** 接続中の配信があるか */
    connected: boolean;
    /** 画面のメッセージ一覧の件数 */
    count: number;
    /** 直近 60 秒に投稿されたコメントの数 */
    perMinute: number;
    /** コメントした人の数 */
    viewers: number;
    /** 「表示中 N / 全 M 件」。隠していなければ null */
    shownLabel: string | null;
    /** 主の配信のリアクション (多い順に 3 つまで) */
    reactions: readonly (readonly [string, number])[];
    /** リアクションの全部 (ヒントと読み上げ用の名前) */
    reactionsLabel: string;
    error: string | null;
    onDismissError: () => void;
    /** 操作パネルを閉じていても出す異常 (セッション切れ・WebSocket の停止など) */
    alerts: readonly StatusAlert[];
    /** 異常を押した (直す場所を開く) */
    onAlert: (alert: StatusAlert) => void;
  }

  let {
    connected,
    count,
    perMinute,
    viewers,
    shownLabel,
    reactions,
    reactionsLabel,
    error,
    onDismissError,
    alerts,
    onAlert,
  }: Props = $props();
</script>

{#if error}
  <div role="alert" class="flex min-h-6 shrink-0 items-start gap-2 bg-error px-3 py-1 text-label text-surface">
    <Icon name="warning" size={14} class="mt-0.5 shrink-0" />
    <span class="min-w-0 flex-1 break-words">{error}</span>
    <button
      type="button"
      aria-label="閉じる"
      title="閉じる"
      onclick={onDismissError}
      class="flex size-5 shrink-0 items-center justify-center rounded-control hover:bg-surface/20"
    >
      <Icon name="close" size={12} />
    </button>
  </div>
{:else}
  <div class="flex shrink-0 flex-col border-t border-outline-variant px-3 text-caption whitespace-nowrap text-fg-3 tabular-nums">
    <!-- 異常は、あるときだけふだんの行の上に 1 行足して並べる (ふだんの行の並びは動かさない)。直るまで出したまま -->
    {#if alerts.length > 0}
      <div class="flex min-h-6 flex-wrap items-center gap-x-3">
        {#each alerts as alert (alert)}
          <button
            type="button"
            data-testid="status-alert"
            onclick={() => onAlert(alert)}
            class="-mx-1 flex items-center gap-1 rounded-control px-1 text-error hover:bg-fg-1/8"
          >
            <Icon name="warning" size={12} />
            {STATUS_ALERT_LABELS[alert]}
          </button>
        {/each}
      </div>
    {/if}
    <div class="flex h-6 items-center gap-3 overflow-hidden">
      <span
        data-testid="status-dot"
        class="size-2 shrink-0 rounded-full {connected ? 'bg-success' : 'bg-fg-3'}"
        aria-hidden="true"
      ></span>
      {#if connected}
        <span>{formatCount(count)} 件</span>
        <span>{formatCount(perMinute)}/分</span>
        <span>{formatCount(viewers)} 人</span>
      {:else}
        <span>未接続</span>
      {/if}
      {#if shownLabel}
        <span class="text-fg-2">{shownLabel}</span>
      {/if}
      {#if reactions.length > 0}
        <span
          data-testid="status-reactions"
          class="ml-auto flex items-center gap-2"
          title={reactionsLabel}
          aria-label={reactionsLabel}
          role="img"
        >
          {#each reactions as [emoji, total] (emoji)}
            <span><span class="grayscale">{emoji}</span> {formatCount(total)}</span>
          {/each}
        </span>
      {/if}
    </div>
  </div>
{/if}
