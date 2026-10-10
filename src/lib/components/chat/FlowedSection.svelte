<script lang="ts">
  /**
   * 流れたものの欄 (docs/specs/02_chat.md「配信中の画面」の「流れたもの」「メッセージ表示仕様」)
   *
   * 流れの画面の外 (上) に出た優先順位 1・2 を、新しいものを上にして残す。
   * - 新しい 2 件は全文の行 (ChatMessage の詰めた形)、それより前は短い札を折り返して並べる (消さない、省略しない、古いほど小さく)
   * - 札の区切りが金額と名前の間と紛れないよう、札の頭に印 (ティアの色の点 / メンバーの点 / 初見) を付け、札の間を広くとる
   * - 高さの上限は親 (ChatDisplay) の高さに対する割合 (既定 1/8)。超えた分はこの欄の中でスクロールする (もう一方の欄を押し出さない)
   * - 上端の境目をドラッグ・ダブルクリック・↑↓ で割合を変え、onHeightChange で保存する (ドラッグは離したときだけ)。計算は flowed-height.ts
   * - 押したコメントが流れに無いとき (表示件数で外れた) は、視聴者情報パネルをその行のすぐ下に広げる (openKey と panel)
   * - 試作は src/routes/design-proto/c
   */
  import type { Snippet } from 'svelte';
  import type { ChatMessage, ViewerInfo } from '$lib/types';
  import type { MessageOrigin } from '$lib/design/priority';
  import { paidLabelOf } from '$lib/design/paid-label';
  import { messageKey, type MessageKey } from '$lib/utils/message-key';
  import { FLOWED_HEIGHT, clampFlowedRatio, resizedFlowedRatio } from '$lib/design/flowed-height';
  import ChatMessageComponent from './ChatMessage.svelte';

  interface Props {
    label: string;
    /** 新しいものが先 */
    items: readonly ChatMessage[];
    fontSize: number;
    showTimestamps: boolean;
    originOf: (message: ChatMessage) => MessageOrigin;
    /** パネル・視聴者のタブで保存し直した読み仮名とメモ (chatStore.infoOf) */
    infoOf: (message: ChatMessage) => ViewerInfo | undefined;
    onClick?: (message: ChatMessage) => void;
    /** 視聴者情報パネルを広げる行 */
    openKey?: MessageKey | null;
    panel?: Snippet;
    /** 高さの上限 (親の高さに対する割合。設定 chat_display.flowed_*_height_ratio) */
    heightRatio?: number;
    /** 境目で割合を変えたとき (保存する) */
    onHeightChange?: (ratio: number) => void;
  }

  let {
    label,
    items,
    fontSize,
    showTimestamps,
    originOf,
    infoOf,
    onClick,
    openKey = null,
    panel,
    heightRatio = FLOWED_HEIGHT.default,
    onHeightChange,
  }: Props = $props();

  /** 全文の行で出す件数 (新しいほうから) */
  const FULL = 2;

  let section = $state<HTMLElement>();
  let heading = $state<HTMLElement>();

  /** ドラッグしている間の状態 (離すまで保存しない) */
  type Drag = { readonly pointerId: number; readonly startY: number; readonly startPx: number; readonly ratio: number };
  let drag = $state<Drag | null>(null);

  /** 今の割合。ドラッグ中はその値、それ以外は設定の値を範囲に収めたもの */
  let ratio = $derived(drag?.ratio ?? clampFlowedRatio(heightRatio));

  /** 高さの下限 = 見出しと 1 行 (いちばん新しい行) の高さ */
  let floorPx = $state(0);
  $effect(() => {
    void items[0];
    void fontSize;
    if (!section || !heading) return;
    const first = section.querySelector<HTMLElement>('[data-message-id]');
    floorPx = heading.offsetHeight + (first?.offsetHeight ?? 0);
  });

  /** 見えている高さ startPx から上へ dragUpPx 動かしたあとの割合 */
  function resized(startPx: number, dragUpPx: number): number {
    return resizedFlowedRatio({
      startPx,
      dragUpPx,
      parentPx: section?.parentElement?.clientHeight ?? 0,
      contentPx: section?.scrollHeight ?? 0,
    });
  }

  function startDrag(event: PointerEvent): void {
    if (!section) return;
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
    drag = { pointerId: event.pointerId, startY: event.clientY, startPx: section.offsetHeight, ratio };
  }

  function moveDrag(event: PointerEvent): void {
    if (!drag || event.pointerId !== drag.pointerId) return;
    drag = { ...drag, ratio: resized(drag.startPx, drag.startY - event.clientY) };
  }

  function endDrag(event: PointerEvent, save: boolean): void {
    if (!drag || event.pointerId !== drag.pointerId) return;
    const next = drag.ratio;
    drag = null;
    if (save) onHeightChange?.(next);
  }

  function handleKey(event: KeyboardEvent): void {
    const direction = event.key === 'ArrowUp' ? 1 : event.key === 'ArrowDown' ? -1 : 0;
    if (direction === 0 || !section) return;
    event.preventDefault();
    const parentPx = section.parentElement?.clientHeight ?? 0;
    onHeightChange?.(resized(section.offsetHeight, direction * FLOWED_HEIGHT.step * parentPx));
  }
</script>

{#if items.length > 0}
  <section
    bind:this={section}
    class="shrink-0 overflow-y-auto border-t border-outline bg-surface-low"
    style="max-height: max({ratio * 100}%, {floorPx}px);"
    aria-label={label}
  >
    <div bind:this={heading} class="sticky top-0 z-10 bg-surface-low">
      <!-- 欄の高さを変える境目 (上へ引くと高く)。ダブルクリックで既定に戻す。
           フォーカスできる separator は ARIA では操作できる部品 (window splitter) だが、Svelte の検査は非対話の役割とみなすので抑える -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div
        role="separator"
        aria-orientation="horizontal"
        aria-label="{label}の高さ"
        aria-valuenow={Math.round(ratio * 100)}
        aria-valuemin={Math.round(FLOWED_HEIGHT.min * 100)}
        aria-valuemax={Math.round(FLOWED_HEIGHT.max * 100)}
        tabindex="0"
        title="ドラッグで高さを変える (ダブルクリックで元に戻す)"
        class="h-1 cursor-row-resize touch-none select-none hover:bg-accent focus-visible:bg-accent {drag ? 'bg-accent' : ''}"
        onpointerdown={startDrag}
        onpointermove={moveDrag}
        onpointerup={(event) => endDrag(event, true)}
        onpointercancel={(event) => endDrag(event, false)}
        ondblclick={() => onHeightChange?.(FLOWED_HEIGHT.default)}
        onkeydown={handleKey}
      ></div>
      <h3 class="px-3 text-caption text-fg-2">
        {label} <span class="tabular-nums">{items.length}</span>
      </h3>
    </div>
    {#each items.slice(0, FULL) as message (messageKey(message))}
      <ChatMessageComponent
        {message}
        {fontSize}
        {showTimestamps}
        compact
        origin={originOf(message)}
        reading={infoOf(message)?.reading}
        notes={infoOf(message)?.notes}
        onClick={() => onClick?.(message)}
      />
      {#if panel && messageKey(message) === openKey}{@render panel()}{/if}
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
