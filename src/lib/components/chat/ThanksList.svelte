<script lang="ts">
  /**
   * 配信の最後のお礼の一覧 (docs/specs/02_chat.md「配信中の画面」の「配信の最後のお礼」)
   *
   * 自分の配信のスパチャ等を時刻順 (古いものが上) に並べる。上に開き、下にコメントの流れを残す (最新も見える)。
   * 高さは親の 1/2 まで。行は ChatMessage の詰めた形 (時刻・札・名前 (読み仮名)・本文)
   */
  import type { ChatMessage } from '$lib/types';
  import type { MessageOrigin } from '$lib/design/priority';
  import { messageKey } from '$lib/utils/message-key';
  import ChatMessageComponent from './ChatMessage.svelte';

  interface Props {
    /** 時刻順 (古いものが先) */
    items: readonly ChatMessage[];
    fontSize: number;
    showTimestamps: boolean;
    originOf: (message: ChatMessage) => MessageOrigin;
    readingOf: (message: ChatMessage) => string | undefined;
    onClick?: (message: ChatMessage) => void;
    onClose: () => void;
  }

  let { items, fontSize, showTimestamps, originOf, readingOf, onClick, onClose }: Props = $props();
</script>

<section class="flex max-h-1/2 shrink-0 flex-col border-b border-outline bg-surface-low" aria-label="この配信のスパチャ等">
  <h2 class="flex h-8 shrink-0 items-center px-3 text-label text-fg-1">
    この配信のスパチャ等 <span class="ml-2 text-caption text-fg-2 tabular-nums">{items.length} 件・時刻順</span>
    <button
      type="button"
      onclick={onClose}
      class="ml-auto rounded-control px-3 py-1 text-label text-fg-2 hover:bg-fg-1/8 active:bg-fg-1/12">閉じる</button
    >
  </h2>
  <ol class="min-h-0 flex-1 overflow-y-auto pb-1">
    {#each items as message (messageKey(message))}
      <li>
        <ChatMessageComponent
          {message}
          {fontSize}
          {showTimestamps}
          compact
          origin={originOf(message)}
          reading={readingOf(message)}
          onClick={() => onClick?.(message)}
        />
      </li>
    {/each}
  </ol>
</section>
