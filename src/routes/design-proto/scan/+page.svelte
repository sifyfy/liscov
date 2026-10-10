<script lang="ts">
  /**
   * 斜め読みしやすいコメントの行の試作 (/design-proto/scan?v=a|b|c&scene=normal|heavy)
   *
   * 今の行 (名前と本文を 1 行に続ける) は、名前の長さで本文の書き出しの位置が行ごとに左右にずれ、
   * 本文を縦にたどる斜め読みで視線が横に振られる (シフィさん、2026-10-10)。本文の頭が縦にそろう並べ方を比べる。
   * 優先順位 1 (スパチャ等の面) はどの案も今の面のまま (もとから見出し + 本文の 2 段) で、2〜5 の行の並べ方だけを変える。
   * - A 名前の行 + 本文: 1 行目に名前・印・#N・時刻を小さく、2 行目に本文。本文の左端はアイコンの右でそろう
   * - B 名前の列: 名前を決まった幅の列に置き、本文はその右の同じ位置から始める。長い名前は列の中で折り返す
   * - C 今の 1 行: 比べる基準 (本体の ChatMessage)
   * - D 名前の列 (右寄せ): B の名前を列の右に寄せ、本文のすぐ左に付ける (短い名前でも名前と本文の間が空かない。IRC 系の並べ方)。
   *   アイコンも列の中で名前のすぐ左に寄せる (左端に残すと短い名前でアイコンと名前が離れる)
   * 各行に data-scan-body (本文) を付け、撮影で本文の書き出しの位置を測る (e2e/design-capture.capture.ts「斜め読みの試作」)
   */
  import { page } from '$app/state';
  import ChatMessage from '$lib/components/chat/ChatMessage.svelte';
  import type { GuiChatMessage } from '$lib/types/generated/GuiChatMessage';
  import { formatTimestamp } from '$lib/utils/format';
  import { feedOf, originOf, priorityOf, stickToBottom } from '../data';

  type Variant = 'a' | 'b' | 'c' | 'd';
  const variant: Variant = (['a', 'b', 'c', 'd'] as const).find((v) => v === page.url.searchParams.get('v')) ?? 'a';
  const scene = page.url.searchParams.get('scene') === 'heavy' ? 'heavy' : 'normal';

  // 登録してある読み仮名とメモ (架空。メモの試作と同じ)
  const NOTES: Readonly<Record<string, string>> = {
    夜ふかし研究会: '攻略情報をよくくれる。ホラーが苦手',
    しおこんぶ: 'ネタバレしがち。注意',
    ねむいペンギン: '赤スパの人。衣装の感想をくれる',
    のりたま: '9/20 のサムネのイラストを描いてくれた。お礼を言う',
    みかん箱: '箱推し',
  };
  const READINGS: Readonly<Record<string, string>> = { 夜ふかし研究会: 'よふかし', のりたま: 'のりたま' };
  const feed = feedOf(scene).map((m) => ({ ...m, author_reading: READINGS[m.author] ?? null, author_notes: NOTES[m.author] ?? null }));

  const initialOf = (m: GuiChatMessage) => [...m.author][0]?.toUpperCase() ?? '?';
</script>

<!-- アイコン (頭文字) -->
{#snippet face(m: GuiChatMessage)}
  <span
    class="mt-px inline-flex size-chat-icon shrink-0 items-center justify-center rounded-full bg-surface-highest text-chat-meta font-bold text-fg-2"
    aria-hidden="true">{initialOf(m)}</span
  >
{/snippet}

<!-- 新規さん・配信内の初回の印 (本体と同じ) -->
{#snippet mark(priority: number)}
  {#if priority === 2}
    <span class="rounded-control bg-highlight px-1 text-chat-meta font-bold text-surface">初見</span>
  {:else if priority === 3}
    <span class="rounded-control border border-highlight px-1 text-chat-meta text-highlight">初見</span>
  {:else if priority === 4}
    <span class="text-chat-meta text-highlight" aria-label="配信内で初めてのコメント">●</span>
  {/if}
{/snippet}

<!-- 名前 (読み仮名)。メンバーは緑 (優先順位 2 の薄い背景の上では通常の色)。size は名前の大きさ (読み仮名は text-chat-meta。em が重ならないよう兄弟にする) -->
{#snippet name(m: GuiChatMessage, priority: number, size = '')}
  <span data-testid="author-name" class="font-bold break-words {size} {m.is_member && priority !== 2 ? 'text-member-text' : 'text-fg-1'}">{m.author}</span>
  {#if m.author_reading}<span class="text-chat-meta text-fg-2">({m.author_reading})</span>{/if}
{/snippet}

{#snippet meta(m: GuiChatMessage, priority: number)}
  <span class="text-chat-meta whitespace-nowrap tabular-nums {priority === 2 ? 'text-fg-2' : 'text-fg-3'}"
    >{#if m.in_stream_comment_count != null}#{m.in_stream_comment_count}&ensp;{/if}{formatTimestamp(m.timestamp)}</span
  >
{/snippet}

{#snippet notes(m: GuiChatMessage)}
  {#if m.author_notes}
    <p class="text-chat-meta break-words text-fg-2">
      <span class="rounded-control border border-current px-1 whitespace-nowrap">メモ</span>
      {m.author_notes}
    </p>
  {/if}
{/snippet}

<!-- A: 1 行目に名前・印・#N・時刻、2 行目に本文 -->
{#snippet rowA(m: GuiChatMessage, priority: number)}
  <div class="flex items-start gap-2 px-3 py-1 text-chat-body text-fg-1 hover:bg-fg-1/8 {priority === 2 ? 'bg-highlight/20' : ''}">
    {@render face(m)}
    <div class="min-w-0 flex-1">
      <p class="flex flex-wrap items-baseline gap-x-1.5">
        {@render mark(priority)}
        {@render name(m, priority, 'text-chat-meta')}
        {@render meta(m, priority)}
      </p>
      <p class="break-words" data-scan-body>{m.content}</p>
      {@render notes(m)}
    </div>
  </div>
{/snippet}

<!-- B・D: 名前の列 (決まった幅) + 本文。D は名前を列の右に寄せる -->
{#snippet rowB(m: GuiChatMessage, priority: number, alignEnd: boolean)}
  <div class="flex items-start gap-2 px-3 py-1.5 text-chat-body text-fg-1 hover:bg-fg-1/8 {priority === 2 ? 'bg-highlight/20' : ''}">
    {#if !alignEnd}{@render face(m)}{/if}
    <p class="flex w-28 shrink-0 flex-wrap items-baseline gap-x-1 break-all {alignEnd ? 'justify-end text-right' : ''}">
      {#if alignEnd}<span class="self-start">{@render face(m)}</span>{/if}
      {@render mark(priority)}
      {@render name(m, priority)}
    </p>
    <div class="min-w-0 flex-1">
      <p class="break-words"><span data-scan-body>{m.content}</span> {@render meta(m, priority)}</p>
      {@render notes(m)}
    </div>
  </div>
{/snippet}

<main class="flex h-screen flex-col bg-surface font-ui text-fg-1">
  <div class="min-h-0 flex-1 overflow-y-auto" data-scan-list {@attach stickToBottom}>
    {#each feed as m (m.id)}
      {@const priority = priorityOf(m)}
      {@const plain = priority !== 1 && m.message_type === 'text' && m.content !== ''}
      <div data-scan-row={plain ? m.content : undefined} style="font-size: 13px;">
        {#if variant === 'c' || priority === 1 || m.message_type !== 'text'}
          <ChatMessage message={m} fontSize={13} showTimestamps origin={originOf(m)} />
        {:else if variant === 'a'}
          {@render rowA(m, priority)}
        {:else}
          {@render rowB(m, priority, variant === 'd')}
        {/if}
      </div>
    {/each}
  </div>
</main>
