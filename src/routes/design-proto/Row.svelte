<script lang="ts">
  /**
   * 試作で共通に使うコメントの 1 行 (docs/design/tokens.md「強調」、ブリーフ §2)
   *
   * - 優先順位 1: スパチャ・ステッカーは YouTube のティアの色、メンバーシップ系は member の色
   * - 優先順位 2: highlight の薄い背景 + 塗りの印「初見」 / 3: 枠だけの印 / 4: 名前の前の点 / 5: 色を持たない
   *   (薄い背景の上では fg-3 が 4.5:1 に届かないので、時刻・回数を fg-2 にする)
   * - 相手の配信 (コラボ) のものは、強さを変えずに配信者名の札で見分ける。相手の配信のスパチャ等は優先順位 5 で、金額を札で添える
   * - compact: 優先順位 1 を面にせず行にする (お礼の一覧など)。長い本文は省略せず折り返す
   * - tone: 文字の明るさ (古いものを控えめにする案で使う)。優先順位 1 には効かない
   */
  import type { GuiChatMessage } from '$lib/types/generated/GuiChatMessage';
  import { originOf, paidLabelOf, priorityOf } from './data';

  type Tone = 'fg-1' | 'fg-2' | 'fg-3';

  let {
    message,
    avatar = true,
    compact = false,
    tone = 'fg-1',
  }: { message: GuiChatMessage; avatar?: boolean; compact?: boolean; tone?: Tone } = $props();

  const TONE_CLASS = { 'fg-1': 'text-fg-1', 'fg-2': 'text-fg-2', 'fg-3': 'text-fg-3' } as const satisfies Record<Tone, string>;

  let priority = $derived(priorityOf(message));
  let partner = $derived(originOf(message) === 'partner');
  let colors = $derived(message.metadata?.superchat_colors ?? null);
  let time = $derived(message.timestamp.slice(0, 5));
  let initial = $derived([...message.author][0] ?? '?');

  /** スパチャ等の札 (金額、またはメンバーシップの種類) */
  let paidLabel = $derived(paidLabelOf(message));
</script>

{#snippet face(cls: string)}
  {#if avatar}
    <span
      class="inline-flex size-5 shrink-0 items-center justify-center rounded-full text-chat-meta font-bold {cls}"
      aria-hidden="true">{initial}</span
    >
  {/if}
{/snippet}

{#snippet source()}
  {#if partner}
    <span class="mr-1 rounded-control bg-surface-highest px-1 text-chat-meta whitespace-nowrap text-fg-2">{message.broadcaster_name}</span>
  {/if}
{/snippet}

{#if priority === 1 && colors && !compact}
  <!-- スパチャ・ステッカー: ティアの色の面 (浮いて見える理由がある数少ない要素なので角丸) -->
  <div class="mx-2 my-1 overflow-hidden rounded-control text-chat-body" style="background: {colors.body_background}; color: {colors.body_text}">
    <div class="flex items-center gap-2 px-3 py-1.5" style="background: {colors.header_background}; color: {colors.header_text}">
      {@render face('bg-surface/30')}
      <span class="min-w-0 font-bold break-words">{message.author}</span>
      <span class="ml-auto shrink-0 text-chat-amount tabular-nums">{message.amount}</span>
    </div>
    {#if message.content}
      <p class="px-3 pt-1 pb-2 break-words">{message.content}</p>
    {:else if message.message_type === 'supersticker'}
      <p class="px-3 pt-1 pb-2">ステッカー</p>
    {/if}
  </div>
{:else if priority === 1 && !compact}
  <!-- メンバーシップ・ギフト・マイルストーン -->
  <div class="mx-2 my-1 overflow-hidden rounded-control bg-member text-chat-body text-on-member">
    <div class="flex items-center gap-2 px-3 py-1.5">
      {@render face('bg-surface/30')}
      <span class="min-w-0 font-bold break-words">{message.author}</span>
      <span class="ml-auto shrink-0 text-chat-meta font-bold">{paidLabel}</span>
    </div>
    {#if message.content}
      <p class="px-3 pt-0 pb-2 break-words">{message.content}</p>
    {/if}
  </div>
{:else if priority === 1}
  <!-- 優先順位 1 を 1 行に畳んだもの (時刻・金額・名前・本文) -->
  <div class="flex items-baseline gap-2 px-3 py-1.5 text-chat-body text-fg-1">
    <span class="shrink-0 text-chat-meta text-fg-3 tabular-nums">{time}</span>
    {#if colors}
      <span
        class="shrink-0 rounded-control px-1.5 text-chat-meta font-bold tabular-nums"
        style="background: {colors.header_background}; color: {colors.header_text}">{message.amount}</span
      >
    {:else}
      <span class="shrink-0 rounded-control bg-member px-1.5 text-chat-meta font-bold text-on-member">{paidLabel}</span>
    {/if}
    <p class="min-w-0 break-words">
      <span class="font-bold">{message.author}</span>
      <span class="ml-1">{message.content}</span>
    </p>
  </div>
{:else}
  <!-- 優先順位 2〜5: 名前と本文を同じ行に続ける -->
  <div class="flex items-start gap-2 px-3 py-1.5 text-chat-body {priority === 2 ? 'bg-highlight/20' : ''} {TONE_CLASS[tone]}">
    {@render face('bg-surface-highest text-fg-2')}
    <p class="min-w-0 break-words">
      {@render source()}
      {#if priority === 2}
        <span class="mr-1 rounded-control bg-highlight px-1 text-chat-meta font-bold text-surface">初見</span>
      {:else if priority === 3}
        <span class="mr-1 rounded-control border border-highlight px-1 text-chat-meta text-highlight">初見</span>
      {:else if priority === 4}
        <span class="mr-0.5 text-chat-meta text-highlight" aria-label="配信内で初めてのコメント">●</span>
      {/if}
      <span class="font-bold">{message.author}</span>
      {#if paidLabel}
        <span class="ml-1 rounded-control border border-outline px-1 text-chat-meta whitespace-nowrap tabular-nums">{paidLabel}</span>
      {/if}
      <span class="ml-1.5">{message.content}</span>
      {#if message.in_stream_comment_count !== null}
        <span class="ml-1.5 text-chat-meta whitespace-nowrap tabular-nums {priority === 2 ? 'text-fg-2' : 'text-fg-3'}"
          >#{message.in_stream_comment_count}</span
        >
      {/if}
      <span class="ml-1 text-chat-meta whitespace-nowrap tabular-nums {priority === 2 ? 'text-fg-2' : 'text-fg-3'}">{time}</span>
    </p>
  </div>
{/if}
