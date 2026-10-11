<script lang="ts">
  /**
   * 視聴者のタブの試作 (段階 5.3b、/design-proto/viewers?v=a|b|c&st=list|edit|delete)
   *
   * 配信の後・準備のときに、視聴者の読み仮名・メモ・タグを整理する画面 (ブリーフ §1「配信の準備・配信の後」、手は空いている)。
   * 中身 (配信者の選択・探す・視聴者の記録・編集・削除) はそろえ、並べ方だけを比べる。540・640・960 の幅で崩れないこと。
   * - A 表: 表のまま、幅に合わせて列を減らす (減らした値は名前の下に回す)。押した行の下で編集する
   * - B 一覧と詳細: 左に名前の一覧、右に選んだ人の記録と編集。狭い幅では詳細が一覧に入れ替わる (「一覧に戻る」)
   * - C 2 行の一覧: 1 行目に名前 (読み仮名)・タグ、2 行目に記録とメモ。押した行の下で編集する
   * st: list = 一覧 / edit = 1 人を編集中 / delete = その人を消す確認
   * 削除の確認文は実際に消える範囲 (06_viewer「視聴者を削除」: プロフィールと読み仮名・メモ・タグがすべて消える) を書く
   */
  import { page } from '$app/state';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { formatCount } from '../data';

  type Variant = 'a' | 'b' | 'c';
  type State = 'list' | 'edit' | 'delete';
  const variant: Variant = (['a', 'b', 'c'] as const).find((v) => v === page.url.searchParams.get('v')) ?? 'a';
  const state: State = (['list', 'edit', 'delete'] as const).find((s) => s === page.url.searchParams.get('st')) ?? 'list';

  type Viewer = Readonly<{
    id: number;
    name: string;
    reading: string | null;
    notes: string | null;
    tags: readonly string[];
    firstSeen: string;
    lastSeen: string;
    comments: number;
    contribution: number;
    member: boolean;
  }>;

  // 架空の視聴者 (最終確認の新しい順。06_viewer の並び)
  const VIEWERS: readonly Viewer[] = [
    { id: 1, name: '夜ふかし研究会', reading: 'よふかしけんきゅうかい', notes: '攻略情報をよくくれる。ホラーが苦手', tags: ['常連', '攻略'], firstSeen: '2026/3/2', lastSeen: '10/9', comments: 1284, contribution: 12000, member: true },
    { id: 2, name: 'しおこんぶ', reading: null, notes: null, tags: [], firstSeen: '2026/5/18', lastSeen: '10/9', comments: 842, contribution: 0, member: false },
    { id: 3, name: 'ねむいペンギン', reading: 'ねむいぺんぎん', notes: '赤スパの人。衣装の感想をくれる', tags: ['常連'], firstSeen: '2026/1/20', lastSeen: '10/9', comments: 516, contribution: 45000, member: true },
    { id: 4, name: 'Kaito_92', reading: 'かいと', notes: '切り抜きから来た (10/9)', tags: [], firstSeen: '10/9', lastSeen: '10/9', comments: 5, contribution: 0, member: false },
    { id: 5, name: 'みかん箱', reading: null, notes: null, tags: [], firstSeen: '2026/8/1', lastSeen: '10/9', comments: 77, contribution: 2000, member: false },
    { id: 6, name: 'とても長い名前のチャンネルでございますよろしくお願いいたします公式', reading: null, notes: 'たまに来る企業の人', tags: ['企業'], firstSeen: '2026/9/12', lastSeen: '10/8', comments: 3, contribution: 0, member: false },
    { id: 7, name: 'のりたま', reading: 'のりたま', notes: null, tags: ['メンバー 12 か月'], firstSeen: '2025/10/1', lastSeen: '10/8', comments: 2210, contribution: 8400, member: true },
    { id: 8, name: 'VeryLongEnglishChannelNameWithoutAnySpacesForTesting', reading: null, notes: null, tags: [], firstSeen: '10/7', lastSeen: '10/7', comments: 1, contribution: 0, member: false },
    { id: 9, name: 'たまごやき', reading: null, notes: 'ネタバレしがち。注意', tags: ['注意'], firstSeen: '2026/2/14', lastSeen: '10/5', comments: 390, contribution: 500, member: false },
    { id: 10, name: 'ひだまり', reading: 'ひだまり', notes: null, tags: [], firstSeen: '10/9', lastSeen: '10/9', comments: 4, contribution: 0, member: false },
    { id: 11, name: 'ゆずぽん', reading: null, notes: null, tags: [], firstSeen: '2026/6/30', lastSeen: '10/1', comments: 61, contribution: 0, member: false },
    { id: 12, name: 'こもれび', reading: null, notes: null, tags: [], firstSeen: '10/9', lastSeen: '10/9', comments: 1, contribution: 0, member: false },
  ];
  const BROADCASTERS = [
    { id: 'UC-proto', name: '試作の配信者', count: 312 },
    { id: 'UC-aoba', name: 'あおば', count: 48 },
  ] as const;

  // 編集・削除の確認をしている人 (一覧のときは誰も)
  const selected = state === 'list' ? null : VIEWERS[0];
  // B の一覧で選ばれている人 (一覧のときも詳細は 1 人目を見せる)
  const detail = VIEWERS[0];

  const yen = (n: number) => (n > 0 ? `¥${formatCount(n)}` : '-');

  const TABS = [
    { icon: 'chat', label: 'チャット' },
    { icon: 'users', label: '視聴者' },
    { icon: 'settings', label: '設定' },
  ] as const;
</script>

<!-- 上の行 (本体の TopBar と同じ見た目。視聴者のタブを選んでいる) -->
{#snippet topBar()}
  <header class="flex h-10 shrink-0 items-center gap-2 border-b border-outline-variant bg-surface-low px-2">
    <nav class="flex shrink-0 items-center gap-0.5" aria-label="画面">
      {#each TABS as tab, i (tab.icon)}
        <button
          type="button"
          aria-label={tab.label}
          class="flex size-8 items-center justify-center rounded-control {i === 1 ? 'bg-accent/15 text-accent' : 'text-fg-2'}"
        >
          <Icon name={tab.icon} size={16} />
        </button>
      {/each}
    </nav>
    <span class="h-5 w-px shrink-0 bg-outline-variant" aria-hidden="true"></span>
    <span class="flex shrink-0 items-center gap-1 rounded-control bg-surface-high py-0.5 pr-0.5 pl-2 text-caption whitespace-nowrap text-fg-1">
      <span class="size-1.5 rounded-full bg-success" aria-hidden="true"></span>
      <span class="font-bold">試作の配信者</span>
      <button type="button" aria-label="試作の配信者 を切断" class="flex size-5 items-center justify-center rounded-control text-fg-2">
        <Icon name="close" size={12} />
      </button>
    </span>
  </header>
{/snippet}

<!-- 配信者の選択・探す・読み込み直す・配信者を削除 -->
{#snippet toolbar()}
  <div class="flex shrink-0 flex-col gap-2 border-b border-outline-variant px-3 py-2">
    <div class="flex flex-wrap items-center gap-2">
      <label class="flex min-w-0 flex-1 items-center gap-2 text-label text-fg-2">
        <span class="shrink-0">配信者</span>
        <select class="min-w-0 flex-1 rounded-control border border-outline bg-surface px-2 py-1 text-label text-fg-1">
          {#each BROADCASTERS as b (b.id)}
            <option>{b.name} ({formatCount(b.count)} 人)</option>
          {/each}
        </select>
      </label>
      <button type="button" class="shrink-0 rounded-control px-2 py-1 text-label whitespace-nowrap text-error hover:bg-fg-1/8">
        この配信者の記録を消す…
      </button>
    </div>
    <div class="flex items-center gap-2">
      <span class="relative min-w-0 flex-1">
        <span class="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-fg-3"><Icon name="search" size={14} /></span>
        <input
          type="search"
          aria-label="視聴者を探す"
          placeholder="名前・読み仮名・メモで探す"
          class="w-full rounded-control border border-outline bg-surface py-1 pr-2 pl-7 text-label text-fg-1 placeholder:text-fg-3"
        />
      </span>
      <button type="button" aria-label="読み込み直す" title="読み込み直す" class="flex size-8 shrink-0 items-center justify-center rounded-control text-fg-2 hover:bg-fg-1/8">
        <Icon name="refresh" size={16} />
      </button>
    </div>
  </div>
{/snippet}

<!-- 名前 (読み仮名)。メンバーは名前を緑に -->
{#snippet nameOf(v: Viewer)}
  <span class="font-bold break-words {v.member ? 'text-member-text' : 'text-fg-1'}">{v.name}</span>
  {#if v.reading}<span class="ml-1 text-caption text-fg-2">({v.reading})</span>{/if}
{/snippet}

<!-- タグ -->
{#snippet tagsOf(v: Viewer)}
  {#each v.tags as tag (tag)}
    <span class="rounded-control border border-outline px-1.5 text-caption whitespace-nowrap text-fg-2">{tag}</span>
  {/each}
{/snippet}

<!-- 記録 (初見・最終・コメント・貢献) -->
{#snippet statsOf(v: Viewer)}
  <span class="tabular-nums">最終 {v.lastSeen}</span>
  <span class="tabular-nums">コメント {formatCount(v.comments)}</span>
  {#if v.contribution > 0}<span class="tabular-nums">{yen(v.contribution)}</span>{/if}
{/snippet}

<!-- 編集 (読み仮名・メモ・タグ・保存・消す) -->
{#snippet editor(v: Viewer)}
  <div class="flex flex-col gap-2">
    <div class="flex flex-wrap gap-2">
      <label class="flex min-w-40 flex-1 flex-col gap-1 text-caption text-fg-2">
        読み仮名
        <input type="text" value={v.reading ?? ''} placeholder="例: やまだ たろう" class="rounded-control border border-outline bg-surface px-2 py-1 text-body text-fg-1 placeholder:text-fg-3" />
      </label>
      <label class="flex min-w-40 flex-1 flex-col gap-1 text-caption text-fg-2">
        タグ (読点・カンマで区切る)
        <input type="text" value={v.tags.join('、')} placeholder="例: 常連、攻略" class="rounded-control border border-outline bg-surface px-2 py-1 text-body text-fg-1 placeholder:text-fg-3" />
      </label>
    </div>
    <label class="flex flex-col gap-1 text-caption text-fg-2">
      メモ
      <textarea rows="2" class="resize-y rounded-control border border-outline bg-surface px-2 py-1 text-body text-fg-1 placeholder:text-fg-3" placeholder="例: 攻略情報をよくくれる">{v.notes ?? ''}</textarea>
    </label>
    {#if state === 'delete'}
      <div role="alert" class="flex flex-col gap-2 rounded-control border border-error bg-error/10 p-2 text-label text-fg-1">
        <p>
          「{v.name}」の記録をすべて消します。読み仮名・メモ・タグと、コメント数などの記録が消えます。取り消せません
          (次にコメントしたら、新しい視聴者として記録しなおします)。
        </p>
        <div class="flex gap-2">
          <button type="button" class="rounded-control bg-error px-3 py-1 text-label text-surface">消す</button>
          <button type="button" class="rounded-control border border-outline px-3 py-1 text-label text-fg-2">やめる</button>
        </div>
      </div>
    {:else}
      <div class="flex flex-wrap items-center gap-2">
        <button type="button" class="rounded-control bg-accent px-3 py-1 text-label text-surface">保存</button>
        <button type="button" class="rounded-control border border-outline px-3 py-1 text-label text-fg-2">やめる</button>
        <button type="button" class="ml-auto rounded-control px-2 py-1 text-label text-error hover:bg-fg-1/8">この視聴者を消す…</button>
      </div>
    {/if}
  </div>
{/snippet}

<!-- ページ送り -->
{#snippet pager()}
  <div class="flex h-9 shrink-0 items-center justify-between border-t border-outline-variant px-3 text-label text-fg-2">
    <button type="button" disabled class="rounded-control px-2 py-1 disabled:opacity-50">前の 50 人</button>
    <span class="tabular-nums">1〜50 人目</span>
    <button type="button" class="rounded-control px-2 py-1 hover:bg-fg-1/8">次の 50 人</button>
  </div>
{/snippet}

<main class="flex h-screen flex-col bg-surface font-ui text-fg-1">
  {@render topBar()}

  {#if variant === 'a'}
    <!-- A: 表。幅に合わせて列を減らし、減らした値は名前の下に回す -->
    {@render toolbar()}
    <div class="min-h-0 flex-1 overflow-y-auto">
      <table class="w-full border-collapse text-label">
        <thead class="sticky top-0 z-10 bg-surface-low text-caption text-fg-2">
          <tr class="text-left">
            <th class="px-3 py-1.5 font-normal">名前 (読み仮名)</th>
            <th class="hidden px-2 py-1.5 text-right font-normal whitespace-nowrap sm:table-cell">最終</th>
            <th class="hidden px-2 py-1.5 text-right font-normal whitespace-nowrap sm:table-cell">コメント</th>
            <th class="hidden px-2 py-1.5 text-right font-normal whitespace-nowrap min-[800px]:table-cell">貢献</th>
            <th class="hidden px-3 py-1.5 font-normal min-[800px]:table-cell">メモ</th>
          </tr>
        </thead>
        <tbody>
          {#each VIEWERS as v (v.id)}
            <tr class="border-t border-outline-variant align-top hover:bg-fg-1/4 {selected?.id === v.id ? 'bg-accent/10' : ''}">
              <td class="px-3 py-1.5">
                {@render nameOf(v)}
                <span class="ml-1 inline-flex flex-wrap gap-1 align-middle">{@render tagsOf(v)}</span>
                <span class="mt-0.5 flex flex-wrap gap-x-3 text-caption text-fg-2 sm:hidden">{@render statsOf(v)}</span>
                {#if v.notes}<span class="mt-0.5 block text-caption break-words text-fg-2 min-[800px]:hidden">{v.notes}</span>{/if}
              </td>
              <td class="hidden px-2 py-1.5 text-right whitespace-nowrap text-fg-2 tabular-nums sm:table-cell">{v.lastSeen}</td>
              <td class="hidden px-2 py-1.5 text-right whitespace-nowrap text-fg-2 tabular-nums sm:table-cell">{formatCount(v.comments)}</td>
              <td class="hidden px-2 py-1.5 text-right whitespace-nowrap text-fg-2 tabular-nums min-[800px]:table-cell">{yen(v.contribution)}</td>
              <td class="hidden px-3 py-1.5 break-words text-fg-2 min-[800px]:table-cell">{v.notes ?? ''}</td>
            </tr>
            {#if selected?.id === v.id}
              <tr>
                <td colspan="5" class="px-3 pb-2">
                  <div class="rounded-control border border-accent/40 bg-surface-low p-2">{@render editor(v)}</div>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>
    {@render pager()}
  {:else if variant === 'b'}
    <!-- B: 一覧と詳細。800px 未満は詳細を開いたら一覧と入れ替える -->
    {@render toolbar()}
    <div class="flex min-h-0 flex-1">
      <ul
        class="min-h-0 overflow-y-auto border-outline-variant min-[800px]:w-80 min-[800px]:shrink-0 min-[800px]:border-r {selected
          ? 'hidden min-[800px]:block'
          : 'flex-1 min-[800px]:flex-none'}"
      >
        {#each VIEWERS as v (v.id)}
          <li>
            <button
              type="button"
              class="flex w-full flex-col gap-0.5 border-b border-outline-variant px-3 py-1.5 text-left text-label hover:bg-fg-1/4 {v.id === detail.id
                ? 'bg-accent/10'
                : ''}"
            >
              <span>{@render nameOf(v)}</span>
              <span class="flex flex-wrap gap-x-3 text-caption text-fg-2">{@render statsOf(v)}</span>
            </button>
          </li>
        {/each}
      </ul>
      <section
        aria-label="視聴者の詳細"
        class="min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3 {selected ? 'flex' : 'hidden min-[800px]:flex'}"
      >
        <button type="button" class="flex items-center gap-1 self-start text-label text-accent min-[800px]:hidden">← 一覧に戻る</button>
        <div class="text-title">{@render nameOf(detail)}</div>
        <div class="flex flex-wrap gap-1">{@render tagsOf(detail)}</div>
        <dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-label sm:grid-cols-4">
          <div><dt class="text-caption text-fg-2">初見</dt><dd class="tabular-nums">{detail.firstSeen}</dd></div>
          <div><dt class="text-caption text-fg-2">最終</dt><dd class="tabular-nums">{detail.lastSeen}</dd></div>
          <div><dt class="text-caption text-fg-2">コメント</dt><dd class="tabular-nums">{formatCount(detail.comments)}</dd></div>
          <div><dt class="text-caption text-fg-2">貢献</dt><dd class="tabular-nums">{yen(detail.contribution)}</dd></div>
        </dl>
        {@render editor(detail)}
      </section>
    </div>
    {@render pager()}
  {:else}
    <!-- C: 2 行の一覧。押した行の下で編集する -->
    {@render toolbar()}
    <ul class="min-h-0 flex-1 overflow-y-auto">
      {#each VIEWERS as v (v.id)}
        <li class="border-b border-outline-variant {selected?.id === v.id ? 'bg-accent/10' : ''}">
          <button type="button" class="flex w-full flex-col gap-0.5 px-3 py-1.5 text-left text-label hover:bg-fg-1/4">
            <span class="flex flex-wrap items-center gap-x-1 gap-y-0.5">{@render nameOf(v)}{@render tagsOf(v)}</span>
            <span class="flex flex-wrap gap-x-3 text-caption text-fg-2">
              {@render statsOf(v)}
              {#if v.notes}<span class="min-w-0 break-words text-fg-1">{v.notes}</span>{/if}
            </span>
          </button>
          {#if selected?.id === v.id}
            <div class="mx-3 mb-2 rounded-control border border-accent/40 bg-surface-low p-2">{@render editor(v)}</div>
          {/if}
        </li>
      {/each}
    </ul>
    {@render pager()}
  {/if}
</main>
