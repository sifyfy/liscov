<script lang="ts">
  /**
   * メモをコメントの行に出す方法の試作 (段階 5.3 の後、/design-proto/memo?v=a|b|c)
   *
   * 視聴者のタブ・視聴者情報パネルで登録したメモ (06_viewer「カスタム情報」) を、配信中にコメントの行で読めるようにする。
   * ブリーフ §2・§5「読み仮名・メモをコメント行に出す」。配信中は手がふさがっているので、操作せずに読めることが前提。
   * 中身 (誰にどんなメモがあるか) はそろえ、出し方だけを比べる。
   * - A 名前の横: 毎回、名前 (読み仮名) のすぐ後ろに「メモ」の札と本文を続ける (段落の中で折り返す)
   * - B 本文の下: 毎回、コメントの下にもう 1 行足して出す
   * - C 最初だけ: その配信でその人の最初のコメントだけ B のように全文を出し、2 回目からは名前の横に「メモ」の札だけ (押すと視聴者情報で読める)
   * 場面は通常の配信。常連 4 人と赤スパの人にメモがあり、1 人は長いメモ
   */
  import { page } from '$app/state';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ChatMessage from '$lib/components/chat/ChatMessage.svelte';
  import type { GuiChatMessage } from '$lib/types/generated/GuiChatMessage';
  import { connections, feedOf, formatCount, originOf, priorityOf, reactions, stats, stickToBottom } from '../data';

  type Variant = 'a' | 'b' | 'c';
  const variant: Variant = (['a', 'b', 'c'] as const).find((v) => v === page.url.searchParams.get('v')) ?? 'a';

  // 登録してあるメモと読み仮名 (架空)
  const NOTES: Readonly<Record<string, string>> = {
    夜ふかし研究会: '攻略情報をよくくれる。ホラーが苦手',
    しおこんぶ: 'ネタバレしがち。注意',
    ねむいペンギン: '赤スパの人。衣装の感想をくれる',
    のりたま: '9/20 のサムネのイラストを描いてくれた。お礼を言う。誕生日は 10/12 なので、次の配信でお祝いする',
    みかん箱: '箱推し',
  };
  const READINGS: Readonly<Record<string, string>> = { 夜ふかし研究会: 'よふかし', のりたま: 'のりたま' };

  const feed = feedOf('normal');
  // C: その人のこの配信で最初のコメント
  const firstIds = new Set([...new Map([...feed].reverse().map((m) => [m.channel_id, m.id])).values()]);

  const notesOf = (m: GuiChatMessage) => NOTES[m.author];
  // 優先順位 1 (面) の上では面の文字色を受け継ぎ、それ以外は控えめな色にする
  const toneOf = (m: GuiChatMessage) => (priorityOf(m) === 1 ? '' : 'text-fg-2');

  const TABS = [
    { icon: 'chat', label: 'チャット' },
    { icon: 'users', label: '視聴者' },
    { icon: 'settings', label: '設定' },
  ] as const;
</script>

<!-- 上の行 (本体の TopBar と同じ見た目) -->
{#snippet topBar()}
  <header class="flex h-10 shrink-0 items-center gap-2 border-b border-outline-variant bg-surface-low px-2">
    <nav class="flex shrink-0 items-center gap-0.5" aria-label="画面">
      {#each TABS as tab, i (tab.icon)}
        <button
          type="button"
          aria-label={tab.label}
          class="flex size-8 items-center justify-center rounded-control {i === 0 ? 'bg-accent/15 text-accent' : 'text-fg-2'}"
        >
          <Icon name={tab.icon} size={16} />
        </button>
      {/each}
    </nav>
    <span class="h-5 w-px shrink-0 bg-outline-variant" aria-hidden="true"></span>
    <div class="flex min-w-0 flex-1 items-center gap-1.5 overflow-x-auto">
      {#each connections.slice(0, 1) as conn (conn.id)}
        <span class="flex shrink-0 items-center gap-1 rounded-control bg-surface-high py-0.5 pr-0.5 pl-2 text-caption whitespace-nowrap text-fg-1">
          <span class="size-1.5 rounded-full bg-success" aria-hidden="true"></span>
          <span class="font-bold">{conn.name}</span>
          <button type="button" aria-label="{conn.name} を切断" class="flex size-5 items-center justify-center rounded-control text-fg-2">
            <Icon name="close" size={12} />
          </button>
        </span>
      {/each}
      <button type="button" aria-label="配信に接続" class="flex size-7 shrink-0 items-center justify-center rounded-control text-fg-2">
        <Icon name="plus" size={16} />
      </button>
    </div>
    <button type="button" aria-label="操作パネルを開く" class="flex size-8 shrink-0 items-center justify-center rounded-control text-fg-2">
      <Icon name="chevron-down" size={16} />
    </button>
  </header>
{/snippet}

<!-- 状態の行 -->
{#snippet statusLine()}
  <div class="flex h-6 shrink-0 items-center gap-3 border-t border-outline-variant px-3 text-caption text-fg-2 tabular-nums">
    <span class="size-2 shrink-0 rounded-full bg-success" aria-hidden="true"></span>
    <span>{formatCount(stats.comments)} 件</span>
    <span>{stats.chatPerMinute}/分</span>
    <span>{formatCount(stats.commenters)} 人</span>
    <span class="ml-auto flex items-center gap-2">
      {#each reactions.counts as r (r.emoji)}
        <span><span class="grayscale">{r.emoji}</span> {r.count}</span>
      {/each}
    </span>
  </div>
{/snippet}

<!-- 「メモ」の札。大きさは囲む文字に合わせる (text-chat-meta の中で text-chat-meta を重ねると em が二重にかかって縮む) -->
{#snippet chip()}
  <span class="rounded-control border border-current px-1 whitespace-nowrap">メモ</span>
{/snippet}

<!-- A: 名前の横に札と本文 -->
{#snippet inline(m: GuiChatMessage)}
  <span class="ml-1.5 text-chat-meta font-normal {toneOf(m)}">{@render chip()} {notesOf(m)}</span>
{/snippet}

<!-- B・C: 本文の下にもう 1 行 -->
{#snippet line(m: GuiChatMessage)}
  <p class="mt-0.5 text-chat-meta break-words {toneOf(m)}">{@render chip()} {notesOf(m)}</p>
{/snippet}

<!-- C の 2 回目から: 札だけ (読むには押して視聴者情報を開く) -->
{#snippet marker(m: GuiChatMessage)}
  <span class="ml-1.5 text-chat-meta font-normal {toneOf(m)}" title={notesOf(m)}>{@render chip()}</span>
{/snippet}

<main class="flex h-screen flex-col bg-surface font-ui text-fg-1">
  {@render topBar()}
  <div class="min-h-0 flex-1 overflow-y-auto" {@attach stickToBottom}>
    {#each feed as m (m.id)}
      {@const note = notesOf(m)}
      {@const first = firstIds.has(m.id)}
      <ChatMessage
        message={m}
        fontSize={13}
        showTimestamps
        origin={originOf(m)}
        reading={READINGS[m.author]}
        afterName={!note ? undefined : variant === 'a' ? inline : variant === 'c' && !first ? marker : undefined}
        below={note && (variant === 'b' || (variant === 'c' && first)) ? line : undefined}
      />
    {/each}
  </div>
  {@render statusLine()}
</main>
