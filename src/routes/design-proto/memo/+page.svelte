<script lang="ts">
  /**
   * メモをコメントの行に出す方法の試作 (段階 5.3 の後、/design-proto/memo)
   *
   * 視聴者のタブ・視聴者情報パネルで登録したメモ (06_viewer「カスタム情報」) を、配信中にコメントの行で読めるようにする。
   * ブリーフ §2・§5「読み仮名・メモをコメント行に出す」。配信中は手がふさがっているので、操作せずに読めることが前提。
   * 3 案 (A 名前の横 / B 本文の下 / C 最初だけ) を比べ、2026-10-10 に B を選んだ (02_chat.md「1 行の形」)。
   * 選んだ後は本体の ChatMessage が出すので、ここでは B だけを本体の部品で見せる。A・C の試作のコードはこのファイルの履歴にある
   * 場面は通常の配信。常連 3 人・赤スパの人・スパチャの人にメモがあり、1 人は長いメモ
   */
  import Icon from '$lib/components/ui/Icon.svelte';
  import ChatMessage from '$lib/components/chat/ChatMessage.svelte';
  import { connections, feedOf, formatCount, originOf, reactions, stats, stickToBottom } from '../data';

  // 登録してあるメモと読み仮名 (架空)
  const NOTES: Readonly<Record<string, string>> = {
    夜ふかし研究会: '攻略情報をよくくれる。ホラーが苦手',
    しおこんぶ: 'ネタバレしがち。注意',
    ねむいペンギン: '赤スパの人。衣装の感想をくれる',
    のりたま: '9/20 のサムネのイラストを描いてくれた。お礼を言う。誕生日は 10/12 なので、次の配信でお祝いする',
    みかん箱: '箱推し',
  };
  const READINGS: Readonly<Record<string, string>> = { 夜ふかし研究会: 'よふかし', のりたま: 'のりたま' };

  // 届いたときに読み仮名・メモが付いている形にする (本体はバックエンドが author_reading・author_notes を付ける)
  const feed = feedOf('normal').map((m) => ({ ...m, author_reading: READINGS[m.author] ?? null, author_notes: NOTES[m.author] ?? null }));

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

<main class="flex h-screen flex-col bg-surface font-ui text-fg-1">
  {@render topBar()}
  <div class="min-h-0 flex-1 overflow-y-auto" {@attach stickToBottom}>
    {#each feed as m (m.id)}
      <ChatMessage message={m} fontSize={13} showTimestamps origin={originOf(m)} />
    {/each}
  </div>
  {@render statusLine()}
</main>
