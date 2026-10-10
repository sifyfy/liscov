<script lang="ts">
  /**
   * 画面の枠の試作 (段階 5.2c、/design-proto/frame?v=a|b|c&st=live|open|empty)
   *
   * 中身 (コメントの流れ・流れたもの・お礼の一覧) は本体の部品をそのまま使い、枠 (操作・状態の置き場所) だけを比べる。
   * 配信中は操作ゼロ (ブリーフ §4)。固定の UI は高さの 15% 以下 (§3)。操作は合間・前後に 1〜2 手。
   * - A 上の 1 行: タブ (アイコン)・接続の札・接続の追加・「表示」を上の 1 行に。押すと下に開く
   * - B 下にまとめる: 上には何も置かない。操作の行を状態の行のすぐ上に置き、押すと上に開く (目は下端にある)
   * - C 準備と配信: 準備の間はすべてを並べて見せ (探さない)、「配信の表示にする」で上の 1 行に畳む
   * - 接続の札: どの接続にも状態の点 (接続中は success)。主の配信 (最初の接続) の名前は太字。分析のタブは外す (ブリーフ §5)
   * st: live = 配信中 (コラボで 2 接続) / open = 操作を開いたところ (A・B は表示の設定、C は準備) / empty = 未接続
   */
  import { page } from '$app/state';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ChatMessage from '$lib/components/chat/ChatMessage.svelte';
  import FlowedSection from '$lib/components/chat/FlowedSection.svelte';
  import { connections, feedOf, formatCount, newcomersOf, originOf, reactions, stats, stickToBottom, supportersOf } from '../data';

  type Variant = 'a' | 'b' | 'c';
  type State = 'live' | 'open' | 'empty';
  const variant: Variant = (['a', 'b', 'c'] as const).find((v) => v === page.url.searchParams.get('v')) ?? 'a';
  const state: State = (['live', 'open', 'empty'] as const).find((s) => s === page.url.searchParams.get('st')) ?? 'live';

  const connected = state !== 'empty';
  const feed = connected ? feedOf('collab') : [];
  const VISIBLE = 24;
  const outOfView = new Set(feed.slice(0, -VISIBLE).map((m) => m.id));
  const supporters = supportersOf(feed);
  const flowedSupporters = [...supporters].reverse().filter((m) => outOfView.has(m.id));
  const flowedNewcomers = newcomersOf(feed).filter((m) => outOfView.has(m.id));
  const noInfo = () => undefined;

  // C: 準備の間は広げる。開いたところ (open) と未接続は準備
  const prepOpen = variant === 'c' && state !== 'live';
  // A・B: 表示の設定を開いたところ
  const displayOpen = variant !== 'c' && state === 'open';

  const TABS = [
    { icon: 'chat', label: 'チャット' },
    { icon: 'users', label: '視聴者' },
    { icon: 'settings', label: '設定' },
  ] as const;
</script>

<!-- タブ (アイコンだけ。名前は title と aria-label) -->
{#snippet tabs()}
  <nav class="flex shrink-0 items-center gap-0.5" aria-label="画面">
    {#each TABS as tab, i (tab.icon)}
      <button
        type="button"
        title={tab.label}
        aria-label={tab.label}
        class="flex size-8 items-center justify-center rounded-control {i === 0 ? 'bg-accent/15 text-accent' : 'text-fg-2 hover:bg-fg-1/8'}"
      >
        <Icon name={tab.icon} size={16} />
      </button>
    {/each}
  </nav>
{/snippet}

<!-- 接続の札 (配信者名と切断)。相手の配信は行の札と同じ見た目 -->
{#snippet chips()}
  <div class="flex min-w-0 flex-wrap items-center gap-1.5">
    {#each connections as conn (conn.id)}
      <span class="flex items-center gap-1 rounded-control bg-surface-high py-0.5 pr-0.5 pl-2 text-label whitespace-nowrap text-fg-1">
        <span class="size-1.5 rounded-full bg-success" title="接続中" aria-hidden="true"></span>
        <span class={conn.origin === 'primary' ? 'font-bold' : ''}>{conn.name}</span>
        <button type="button" aria-label="{conn.name} を切断" title="切断" class="flex size-5 items-center justify-center rounded-control text-fg-2 hover:bg-fg-1/8">
          <Icon name="close" size={12} />
        </button>
      </span>
    {/each}
    <button type="button" aria-label="配信をつなぐ" title="配信をつなぐ" class="flex size-7 items-center justify-center rounded-control text-fg-2 hover:bg-fg-1/8">
      <Icon name="plus" size={16} />
    </button>
  </div>
{/snippet}

<!-- 表示の設定 (A・B は開いたときだけ、C は準備の中にいつも) -->
{#snippet displaySettings()}
  <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-label text-fg-1">
    <span class="flex items-center gap-1">
      文字
      <button type="button" aria-label="文字を小さく" class="flex size-7 items-center justify-center rounded-control border border-outline"><Icon name="minus" size={14} /></button>
      <span class="w-8 text-center text-caption text-fg-2 tabular-nums">13px</span>
      <button type="button" aria-label="文字を大きく" class="flex size-7 items-center justify-center rounded-control border border-outline"><Icon name="plus" size={14} /></button>
    </span>
    <label class="flex items-center gap-1.5"><input type="checkbox" checked class="accent-accent" />時刻</label>
    <label class="flex items-center gap-1.5">
      表示
      <select class="rounded-control border border-outline bg-surface px-1.5 py-1 text-label">
        <option>無制限</option>
        <option>新しい 500 件</option>
      </select>
    </label>
    <span class="flex items-center gap-1.5">
      <label class="flex items-center gap-1"><input type="checkbox" checked class="accent-accent" />通常</label>
      <label class="flex items-center gap-1"><input type="checkbox" checked class="accent-accent" />スパチャ等</label>
      <label class="flex items-center gap-1"><input type="checkbox" checked class="accent-accent" />メンバー</label>
    </span>
    <input type="search" placeholder="コメントを探す" class="min-w-0 flex-1 rounded-control border border-outline bg-surface px-2 py-1 text-label placeholder:text-fg-3" />
    <button type="button" class="rounded-control border border-outline px-3 py-1 text-label text-error hover:bg-fg-1/8">コメントを消す</button>
  </div>
{/snippet}

<!-- 状態の行 (異常のときだけ目立つ。仕様「状態」) -->
{#snippet status()}
  <div class="flex h-6 shrink-0 items-center gap-3 border-t border-outline-variant px-3 text-caption text-fg-3 tabular-nums">
    {#if connected}
      <span class="size-2 shrink-0 rounded-full bg-success" title="接続中"></span>
      <span>{formatCount(stats.comments)} 件</span>
      <span>{stats.chatPerMinute}/分</span>
      <span>{formatCount(stats.commenters)} 人</span>
      <span class="ml-auto flex items-center gap-2">
        {#each reactions.counts as r (r.emoji)}
          <span><span class="grayscale">{r.emoji}</span> {r.count}</span>
        {/each}
      </span>
    {:else}
      <span class="size-2 shrink-0 rounded-full bg-fg-3"></span>
      <span>未接続</span>
    {/if}
  </div>
{/snippet}

<!-- 配信をつなぐ (未接続のときは流れの場所に出す。仕様「状態」) -->
{#snippet connectForm()}
  <div class="flex w-full max-w-md flex-col gap-2">
    <label for="url" class="text-label text-fg-1">配信の URL</label>
    <div class="flex gap-2">
      <input id="url" type="text" placeholder="https://www.youtube.com/watch?v=..." class="min-w-0 flex-1 rounded-control border border-outline bg-surface px-2 py-1.5 text-body placeholder:text-fg-3" />
      <button type="button" class="shrink-0 rounded-control bg-accent px-4 py-1.5 text-label whitespace-nowrap text-surface">つなぐ</button>
    </div>
    <label class="flex items-center gap-1.5 text-label text-fg-2"><input type="checkbox" class="accent-accent" />すべてのコメントを取る (トップのチャットにしない)</label>
  </div>
{/snippet}

<!-- 中身: 流れ・流れたもの・お礼の一覧を開くボタン -->
{#snippet body()}
  {#if connected}
    <div class="min-h-0 flex-1 overflow-y-auto" {@attach stickToBottom}>
      {#each feed as m (m.id)}
        <ChatMessage message={m} fontSize={13} showTimestamps origin={originOf(m)} />
      {/each}
    </div>
    <FlowedSection label="流れた新規さん" items={flowedNewcomers} fontSize={13} showTimestamps {originOf} infoOf={noInfo} />
    <FlowedSection label="流れたスパチャ等" items={flowedSupporters} fontSize={13} showTimestamps {originOf} infoOf={noInfo} />
    <button type="button" class="h-7 shrink-0 border-t border-outline-variant bg-surface-low px-3 text-left text-label text-fg-2 hover:bg-fg-1/8">
      この配信のスパチャ等の一覧 <span class="tabular-nums">{supporters.length}</span>
    </button>
  {:else}
    <div class="flex flex-1 items-center justify-center p-4">{@render connectForm()}</div>
  {/if}
{/snippet}

<main class="relative flex h-screen flex-col bg-surface font-ui text-fg-1">
  {#if variant === 'a'}
    <!-- A: 上の 1 行 -->
    <header class="relative flex shrink-0 items-center gap-2 border-b border-outline-variant px-2 py-1">
      {@render tabs()}
      <span class="h-5 w-px shrink-0 bg-outline-variant" aria-hidden="true"></span>
      {#if connected}{@render chips()}{/if}
      <button
        type="button"
        class="ml-auto flex shrink-0 items-center gap-1 rounded-control px-2 py-1 text-label whitespace-nowrap {displayOpen ? 'bg-accent/15 text-accent' : 'text-fg-2 hover:bg-fg-1/8'}"
      >
        <Icon name="filter" size={14} />表示
      </button>
      {#if displayOpen}
        <div class="absolute top-full right-2 left-2 z-20 mt-1 rounded-popup bg-surface-high p-3 shadow-elevation-2">{@render displaySettings()}</div>
      {/if}
    </header>
    {@render body()}
    {@render status()}
  {:else if variant === 'b'}
    <!-- B: 下にまとめる (上には何も置かない) -->
    {@render body()}
    <div class="relative flex shrink-0 items-center gap-2 border-t border-outline-variant px-2 py-1">
      {@render tabs()}
      <span class="h-5 w-px shrink-0 bg-outline-variant" aria-hidden="true"></span>
      {#if connected}{@render chips()}{/if}
      <button
        type="button"
        class="ml-auto flex shrink-0 items-center gap-1 rounded-control px-2 py-1 text-label whitespace-nowrap {displayOpen ? 'bg-accent/15 text-accent' : 'text-fg-2 hover:bg-fg-1/8'}"
      >
        <Icon name="filter" size={14} />表示
      </button>
      {#if displayOpen}
        <div class="absolute right-2 bottom-full left-2 z-20 mb-1 rounded-popup bg-surface-high p-3 shadow-elevation-2">{@render displaySettings()}</div>
      {/if}
    </div>
    {@render status()}
  {:else}
    <!-- C: 準備と配信 -->
    {#if prepOpen}
      <header class="flex shrink-0 flex-col gap-3 border-b border-outline-variant bg-surface-low px-3 py-2">
        <div class="flex items-center gap-2">
          {@render tabs()}
          <span class="ml-1 text-label text-fg-2">準備</span>
          {#if connected}
            <button type="button" class="ml-auto shrink-0 rounded-control bg-accent px-3 py-1 text-label whitespace-nowrap text-surface">配信の表示にする</button>
          {/if}
        </div>
        {#if connected}
          <ul class="flex flex-col gap-1">
            {#each connections as conn (conn.id)}
              <li class="flex items-center gap-2 rounded-control bg-surface px-2 py-1 text-label">
                <span class="size-1.5 shrink-0 rounded-full bg-success" title="接続中" aria-hidden="true"></span>
                <span class="shrink-0 font-bold">{conn.name}</span>
                <span class="min-w-0 break-words text-fg-2">{conn.title}</span>
                <button type="button" class="ml-auto shrink-0 rounded-control px-2 py-0.5 text-label text-fg-2 hover:bg-fg-1/8">切断</button>
              </li>
            {/each}
          </ul>
          {@render connectForm()}
        {/if}
        {@render displaySettings()}
      </header>
    {:else}
      <header class="flex shrink-0 items-center gap-2 border-b border-outline-variant px-2 py-1">
        {@render tabs()}
        <span class="h-5 w-px shrink-0 bg-outline-variant" aria-hidden="true"></span>
        <span class="flex min-w-0 flex-wrap items-center gap-x-3 text-label text-fg-2">
          {#each connections as conn (conn.id)}<span class="flex items-center gap-1 whitespace-nowrap"
              ><span class="size-1.5 rounded-full bg-success" title="接続中" aria-hidden="true"></span><span
                class={conn.origin === 'primary' ? 'font-bold text-fg-1' : ''}>{conn.name}</span
              ></span
            >{/each}
        </span>
        <button type="button" class="ml-auto shrink-0 rounded-control px-2 py-1 text-label whitespace-nowrap text-fg-2 hover:bg-fg-1/8">準備に戻す</button>
      </header>
    {/if}
    {@render body()}
    {@render status()}
  {/if}
</main>
