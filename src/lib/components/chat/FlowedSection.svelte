<script lang="ts">
  /**
   * 流れたものの欄 (docs/specs/02_chat.md「配信中の画面」の「流れたもの」「メッセージ表示仕様」)
   *
   * 流れの画面の外 (上) に出た優先順位 1・2 を、新しいものを上にして残す。
   * - 新しい 2 件は全文の行 (ChatMessage の詰めた形)、それより前は短い札を折り返して並べる (消さない、省略しない、古いほど小さく)
   * - 札の区切りが金額と名前の間と紛れないよう、札の頭に印 (ティアの色の点 / メンバーの点 / 初見) を付け、札の間を広くとる
   * - 高さは親の 1/8 まで。超えた分はこの欄の中でスクロールする (もう一方の欄を押し出さない)
   * - 試作は src/routes/design-proto/c
   */
  import type { ChatMessage } from '$lib/types';
  import type { MessageOrigin } from '$lib/design/priority';
  import { paidLabelOf } from '$lib/design/paid-label';
  import { messageKey } from '$lib/utils/message-key';
  import ChatMessageComponent from './ChatMessage.svelte';

  interface Props {
    label: string;
    /** 新しいものが先 */
    items: readonly ChatMessage[];
    fontSize: number;
    showTimestamps: boolean;
    originOf: (message: ChatMessage) => MessageOrigin;
    readingOf: (message: ChatMessage) => string | undefined;
    onClick?: (message: ChatMessage) => void;
  }

  let { label, items, fontSize, showTimestamps, originOf, readingOf, onClick }: Props = $props();

  /** 全文の行で出す件数 (新しいほうから) */
  const FULL = 2;
</script>

{#if items.length > 0}
  <section class="max-h-1/8 shrink-0 overflow-y-auto border-t border-outline bg-surface-low" aria-label={label}>
    <h3 class="sticky top-0 z-10 bg-surface-low px-3 pt-1 text-caption text-fg-2">
      {label} <span class="tabular-nums">{items.length}</span>
    </h3>
    {#each items.slice(0, FULL) as message (messageKey(message))}
      <ChatMessageComponent
        {message}
        {fontSize}
        {showTimestamps}
        compact
        origin={originOf(message)}
        reading={readingOf(message)}
        onClick={() => onClick?.(message)}
      />
    {/each}
    {#if items.length > FULL}
      <p class="flex flex-wrap gap-x-4 gap-y-1 px-3 pt-0.5 pb-1.5 text-chat-meta text-fg-2" style="font-size: {fontSize}px;">
        {#each items.slice(FULL) as message (messageKey(message))}
          {@const paid = paidLabelOf(message)}
          {@const colors = message.metadata?.superchat_colors}
          <span data-testid="flowed-chip" class="whitespace-nowrap">
            {#if paid}
              <span
                class="mr-0.5 inline-block size-2 rounded-full {colors ? '' : 'bg-member'}"
                style={colors ? `background-color: ${colors.header_background};` : undefined}
                aria-hidden="true"
              ></span><span class="tabular-nums">{paid}</span>
            {:else}
              <span class="text-highlight">初見</span>
            {/if}
            <span class="ml-0.5 text-fg-1">{message.author}</span>
          </span>
        {/each}
      </p>
    {/if}
  </section>
{/if}
