<script lang="ts">
  /**
   * 方向性の試作 A「帯型」(/design-proto/a?scene=...)
   *
   * 軸: 1 本の流れの上に、細い帯を 2 本だけ足す。
   * - スパチャ等の帯: 配信全体のもの (自分の配信) を新しい順に左から並べ、消さない。新しい 3 件はティアの色、それより古いものは控えめ
   * - 新規さんの帯: 最新の新規さんの初回コメントを 1 行。スパチャ等とは別枠
   * - 配信の最後のお礼: 帯の「一覧」を押す (1 手) と、時刻順の一覧が上半分に開き、下半分に最新のコメントが残る
   * - 参加型: 参加中・待機の 2 行を帯の下に足す
   */
  import { page } from '$app/state';
  import Row from '../Row.svelte';
  import Status from '../Status.svelte';
  import Party from '../Party.svelte';
  import { feedOf, newcomersOf, sceneOf, stickToBottom, supportersOf } from '../data';

  const scene = sceneOf(page.url);
  const feed = feedOf(scene);
  const supporters = supportersOf(feed);
  const newcomers = newcomersOf(feed);
  let thanksOpen = $state(scene === 'thanks');

  /** 帯で目立たせる件数 (新しいほうから) */
  const VIVID = 3;
</script>

<main class="flex h-screen flex-col bg-surface font-ui text-fg-1" style="font-size: 13px">
  <Status class="h-7 border-b border-outline-variant" />

  <!-- スパチャ等の帯 (新しい順、消さない) -->
  <div class="flex h-10 shrink-0 items-center gap-1.5 border-b border-outline-variant pl-2">
    <div class="flex min-w-0 flex-1 gap-1.5 overflow-hidden">
      {#each [...supporters].reverse() as m, i (m.id)}
        {@const colors = m.metadata?.superchat_colors}
        {@const label = m.amount ?? (m.message_type === 'membership_gift' ? `ギフト ×${m.metadata?.gift_count}` : 'メンバー')}
        {#if i < VIVID}
          <span
            class="inline-flex h-7 max-w-44 shrink-0 items-center gap-1.5 rounded-full px-2.5 text-label tabular-nums {colors
              ? ''
              : 'bg-member text-on-member'}"
            style={colors ? `background: ${colors.header_background}; color: ${colors.header_text}` : ''}
          >
            <span class="shrink-0">{label}</span>
            <span class="min-w-0 truncate font-normal">{m.author}</span>
          </span>
        {:else}
          <span class="inline-flex h-7 max-w-36 shrink-0 items-center gap-1.5 rounded-full border border-outline-variant px-2.5 text-caption text-fg-2 tabular-nums">
            <span class="shrink-0">{label}</span>
            <span class="min-w-0 truncate">{m.author}</span>
          </span>
        {/if}
      {/each}
    </div>
    <button
      onclick={() => (thanksOpen = !thanksOpen)}
      class="mr-2 shrink-0 rounded-control px-2 py-1 text-label text-fg-2 hover:bg-fg-1/8 active:bg-fg-1/12"
      aria-expanded={thanksOpen}
    >
      一覧 <span class="tabular-nums">{supporters.length}</span>
    </button>
  </div>

  <!-- 新規さんの帯 (別枠) -->
  {#if newcomers.length > 0}
    {@const latest = newcomers[0]}
    <div class="flex h-8 shrink-0 items-center gap-2 border-b border-outline-variant bg-highlight/20 px-3 text-chat-body">
      <span class="shrink-0 rounded-control bg-highlight px-1 text-chat-meta font-bold text-surface">初見</span>
      <p class="min-w-0 truncate">
        <span class="font-bold">{latest.author}</span>
        <span class="ml-1.5">{latest.content}</span>
      </p>
      <span class="ml-auto shrink-0 text-chat-meta text-fg-2 tabular-nums">{latest.timestamp}</span>
      {#if newcomers.length > 1}
        <span class="shrink-0 text-chat-meta text-fg-2">ほか {newcomers.length - 1} 人</span>
      {/if}
    </div>
  {/if}

  {#if scene === 'party'}
    <div class="shrink-0 border-b border-outline-variant"><Party /></div>
  {/if}

  <!-- 配信の最後のお礼: 時刻順の一覧 (上半分) -->
  {#if thanksOpen}
    <section class="flex h-1/2 shrink-0 flex-col border-b border-outline bg-surface-low" aria-label="スパチャ等の一覧">
      <h2 class="flex h-8 shrink-0 items-center px-3 text-label">
        この配信のスパチャ等 <span class="ml-2 text-caption text-fg-2 tabular-nums">{supporters.length} 件・時刻順</span>
      </h2>
      <ol class="min-h-0 flex-1 overflow-y-auto">
        {#each supporters as m (m.id)}
          <li><Row message={m} compact avatar={false} /></li>
        {/each}
      </ol>
    </section>
  {/if}

  <div class="min-h-0 flex-1 overflow-y-auto" {@attach stickToBottom}>
    {#each feed as m (m.id)}
      <Row message={m} />
    {/each}
  </div>
</main>
