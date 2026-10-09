<script lang="ts">
  /**
   * 視聴者情報パネルの置き方の試作 (段階 5.3、/design-proto/viewer?v=a|b|c)
   *
   * コメントを押して視聴者の読み仮名・メモを登録するところ。ブリーフ §4「コメントから 2 手で入力を始められる。
   * 配信中でも流れを止めずにできる」。中身 (読み仮名・メモ・その人のコメント) はそろえ、置き方だけを比べる。
   * どの案でも、開いたら読み仮名の欄にすぐ打てる (1 手目で開き、そのまま打つ) とし、Enter で保存する。
   * - A 右に置く: 幅があれば流れの右に並べ (流れを覆わない)、狭ければ流れの右側に重ねる。上の行と状態の行は覆わない
   * - B 下に開く: 流れの下 (状態の行のすぐ上) に開く。流れは幅いっぱいのまま、高さだけ縮む
   * - C 行の中で広げる: 押した行のすぐ下に入力欄を広げる。流れは止まり (最新から離れる)、打っている間は戻らない (「最新へ」に止まっている間のスパチャ等・新規さんの件数)
   * 場面は配信中 (コメントが流れている最中) に、読めない名前に気づいて読み仮名を入れるところ
   */
  import { page } from '$app/state';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ChatMessage from '$lib/components/chat/ChatMessage.svelte';
  import FlowedSection from '$lib/components/chat/FlowedSection.svelte';
  import LatestButton from '$lib/components/chat/LatestButton.svelte';
  import { connections, feedOf, formatCount, newcomersOf, originOf, reactions, stats, stickToBottom, supportersOf } from '../data';

  type Variant = 'a' | 'b' | 'c';
  const variant: Variant = (['a', 'b', 'c'] as const).find((v) => v === page.url.searchParams.get('v')) ?? 'a';

  const feed = feedOf('normal');
  const VISIBLE = 24;
  const outOfView = new Set(feed.slice(0, -VISIBLE).map((m) => m.id));
  const supporters = supportersOf(feed);
  const flowedSupporters = [...supporters].reverse().filter((m) => outOfView.has(m.id));
  const flowedNewcomers = newcomersOf(feed).filter((m) => outOfView.has(m.id));
  const noReading = () => undefined;

  // 押したコメント (読みにくい名前の常連が、役に立つことを書いたところ)
  const clicked = feed.find((m) => m.content === 'ここのボス、最初は足を狙うと楽')!;
  const viewer = { name: clicked.author, channelId: clicked.channel_id };
  const history = [...feed.filter((m) => m.channel_id === viewer.channelId)].reverse();
  // 打っている途中 (まだ保存していない)
  const draft = { reading: 'よふかしけんきゅうかい', notes: '' };

  const now = Date.now();
  const focusOnOpen = (node: HTMLInputElement) => node.focus();
  // C: 押した行が画面の中ほどに来るようにする (流れは止まっている)
  const centerClicked = (node: HTMLElement) => {
    node.querySelector('[data-clicked]')?.scrollIntoView({ block: 'center' });
  };

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

<!-- 流れの 1 行 (押した行は強調) -->
{#snippet row(m: (typeof feed)[number])}
  <div data-clicked={m.id === clicked.id ? '' : undefined}>
    <ChatMessage message={m} fontSize={13} showTimestamps origin={originOf(m)} highlighted={m.id === clicked.id} />
  </div>
{/snippet}

<!-- 流れたもの・お礼の一覧を開くボタン -->
{#snippet flowedAndThanks()}
  <FlowedSection label="流れた新規さん" items={flowedNewcomers} fontSize={13} showTimestamps {originOf} readingOf={noReading} />
  <FlowedSection label="流れたスパチャ等" items={flowedSupporters} fontSize={13} showTimestamps {originOf} readingOf={noReading} />
  <button type="button" class="h-7 shrink-0 border-t border-outline-variant bg-surface-low px-3 text-left text-label text-fg-2">
    この配信のスパチャ等の一覧 <span class="tabular-nums">{supporters.length}</span>
  </button>
{/snippet}

<!-- 誰の情報か (アイコン・名前・閉じる) -->
{#snippet identity(size: 'normal' | 'small')}
  <div class="flex items-center gap-2">
    <span
      class="flex shrink-0 items-center justify-center rounded-full bg-surface-highest font-bold text-fg-2 {size === 'normal'
        ? 'size-9 text-body'
        : 'size-6 text-caption'}"
      aria-hidden="true">{viewer.name.slice(0, 1)}</span
    >
    <span class="min-w-0 flex-1">
      <span class="block text-label font-bold break-words text-fg-1">{viewer.name}</span>
      {#if size === 'normal'}
        <span class="block text-caption break-all text-fg-3">{viewer.channelId}</span>
      {/if}
    </span>
    <button type="button" aria-label="閉じる" title="閉じる (Esc)" class="flex size-7 shrink-0 items-center justify-center rounded-control text-fg-2 hover:bg-fg-1/8">
      <Icon name="close" size={14} />
    </button>
  </div>
{/snippet}

<!-- 読み仮名・メモ・保存。開いたら読み仮名の欄にすぐ打てる -->
{#snippet editor(layout: 'stack' | 'row')}
  <div class="flex flex-col gap-2">
    <div class="flex gap-2 {layout === 'row' ? 'flex-wrap' : 'flex-col'}">
      <label class="flex min-w-48 flex-1 flex-col gap-1 text-caption text-fg-2">
        読み仮名
        <input
          type="text"
          value={draft.reading}
          placeholder="例: やまだ たろう"
          {@attach focusOnOpen}
          class="rounded-control border border-outline bg-surface px-2 py-1 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent focus:outline-2 focus:outline-accent/30"
        />
      </label>
      <label class="flex min-w-48 flex-1 flex-col gap-1 text-caption text-fg-2">
        メモ
        <input
          type="text"
          value={draft.notes}
          placeholder="例: 攻略情報をよくくれる"
          class="rounded-control border border-outline bg-surface px-2 py-1 text-body text-fg-1 placeholder:text-fg-3"
        />
      </label>
    </div>
    <div class="flex items-center gap-2">
      <button type="button" class="rounded-control bg-accent px-3 py-1 text-label text-surface">保存</button>
      <span class="text-caption text-fg-3">Enter で保存</span>
    </div>
  </div>
{/snippet}

<!-- その人のこの配信のコメント (新しい順) -->
{#snippet historyList()}
  <div class="flex min-h-0 flex-1 flex-col">
    <h3 class="shrink-0 px-3 pt-2 pb-1 text-caption text-fg-2">この配信のコメント <span class="tabular-nums">{history.length}</span> 件</h3>
    <div class="min-h-0 flex-1 overflow-y-auto">
      {#each history as m (m.id)}
        <ChatMessage message={m} fontSize={13} showTimestamps compact origin={originOf(m)} highlighted={m.id === clicked.id} />
      {/each}
    </div>
  </div>
{/snippet}

<main class="flex h-screen flex-col bg-surface font-ui text-fg-1">
  {@render topBar()}

  {#if variant === 'a'}
    <!-- A: 右に置く。幅があれば並べ、狭ければ流れの右側に重ねる -->
    <div class="relative flex min-h-0 flex-1">
      <div class="flex min-w-0 flex-1 flex-col">
        <div class="min-h-0 flex-1 overflow-y-auto" {@attach stickToBottom}>
          {#each feed as m (m.id)}{@render row(m)}{/each}
        </div>
        {@render flowedAndThanks()}
      </div>
      <aside
        aria-label="視聴者情報"
        class="absolute inset-y-0 right-0 flex w-72 flex-col border-l border-outline-variant bg-surface-low shadow-elevation-2 min-[800px]:static min-[800px]:shadow-none"
      >
        <div class="flex shrink-0 flex-col gap-3 border-b border-outline-variant p-3">
          {@render identity('normal')}
          {@render editor('stack')}
        </div>
        {@render historyList()}
      </aside>
    </div>
  {:else if variant === 'b'}
    <!-- B: 下に開く。流れは幅いっぱいのまま、高さだけ縮む -->
    <div class="min-h-0 flex-1 overflow-y-auto" {@attach stickToBottom}>
      {#each feed as m (m.id)}{@render row(m)}{/each}
    </div>
    {@render flowedAndThanks()}
    <section aria-label="視聴者情報" class="flex max-h-1/2 shrink-0 flex-col border-t border-outline bg-surface-low">
      <div class="flex shrink-0 flex-col gap-2 px-3 pt-2 pb-2">
        {@render identity('small')}
        {@render editor('row')}
      </div>
      {@render historyList()}
    </section>
  {:else}
    <!-- C: 行の中で広げる。流れは止まり、「最新へ」の帯が出る -->
    <div class="min-h-0 flex-1 overflow-y-auto" {@attach centerClicked}>
      {#each feed as m (m.id)}
        {@render row(m)}
        {#if m.id === clicked.id}
          <section aria-label="視聴者情報" class="mx-2 my-1 flex flex-col gap-2 rounded-control border border-accent/40 bg-surface-low p-2">
            {@render identity('small')}
            {@render editor('row')}
            <button type="button" class="self-start text-caption text-accent hover:underline">
              この配信のコメント <span class="tabular-nums">{history.length}</span> 件を見る
            </button>
          </section>
        {/if}
      {/each}
    </div>
    <div class="flex shrink-0 justify-end px-3 py-1">
      <LatestButton deadline={null} {now} unread={{ supporters: 1, newcomers: 1 }} onClick={() => {}} />
    </div>
    {@render flowedAndThanks()}
  {/if}

  {@render statusLine()}
</main>
