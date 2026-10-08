<script lang="ts">
  /**
   * デザイントークンの見本ページ (docs/design/tokens.md)
   *
   * 全トークンを並べ、撮影 (pnpm design:capture) してシフィさんが値を判断するための画面。
   * アプリのナビゲーションからは開けない。/design-tokens に直接移動して開く。
   * ここでは --sys-* と Tailwind に登録した名前だけを使う (生の色値・任意値を使わない)。
   * コメントの見本の font-size: 13px は、利用者の設定 (message_font_size) の既定の代わり。
   */

  type Row = Readonly<{ name: string; body: string; meta: string }>;

  const SAMPLE = '初見です！よろしくお願いします 0123456789 ¥10,000 さっきの謎解き、本棚の順番は左から赤・青・緑';

  const SURFACES = ['surface', 'surface-low', 'surface-mid', 'surface-high', 'surface-highest'] as const;

  const TYPE_SCALE = [
    { cls: 'text-caption', label: 'caption 12/16' },
    { cls: 'text-label', label: 'label 13/18 700' },
    { cls: 'text-body', label: 'body 14/20' },
    { cls: 'text-title', label: 'title 16/24 700' },
    { cls: 'text-display', label: 'display 20/28 700' },
  ] as const;

  // 優先順位 2〜5 の行の見本 (名前・本文は架空)
  const rows = {
    first: { name: 'みかん箱', body: '初見です！よろしくお願いします', meta: '#1' },
    newRepeat: { name: 'Kaito_92', body: 'さっきのところ右の道だったかも', meta: '#3' },
    returning: { name: '夜ふかし研究会', body: 'こんばんは！', meta: '#1' },
    normal: { name: 'しおこんぶ', body: 'このボス強すぎない？', meta: '#12' },
  } as const satisfies Record<string, Row>;
</script>

<main class="min-h-screen bg-surface p-6 font-ui text-fg-1">
  <h1 class="text-title">デザイントークンの見本</h1>
  <p class="mt-1 text-body text-fg-2">docs/design/tokens.md の値。ダークとライトは data-theme で切り替える。</p>

  <!-- 書体 -->
  <section class="mt-8">
    <h2 class="text-label text-fg-2">書体</h2>
    <div class="mt-3 grid gap-4">
      <div>
        <p class="text-caption text-fg-3">BIZ UDPゴシック</p>
        <p class="font-ui text-body">{SAMPLE}</p>
        <p class="font-ui text-body font-bold">{SAMPLE}</p>
      </div>
    </div>
  </section>

  <!-- 文字の段階 -->
  <section class="mt-8">
    <h2 class="text-label text-fg-2">文字の段階 (アプリの UI)</h2>
    <div class="mt-3 grid gap-2">
      {#each TYPE_SCALE as t (t.cls)}
        <div class="flex items-baseline gap-4">
          <span class="w-40 shrink-0 text-caption text-fg-3">{t.label}</span>
          <span class={t.cls}>コメント 1,234 件・同接 567</span>
        </div>
      {/each}
    </div>
  </section>

  <!-- コメントの段階 (既定の 13px) -->
  <section class="mt-8">
    <h2 class="text-label text-fg-2">文字の段階 (コメント、基準 13px)</h2>
    <div class="mt-3 bg-surface-low" style="font-size: 13px">
      {#each Object.values(rows) as r (r.name)}
        <div class="border-b border-outline-variant px-3 py-1 text-chat-body">
          <span class="font-bold">{r.name}</span>
          <span class="ml-2">{r.body}</span>
          <span class="ml-2 text-chat-meta text-fg-3 tabular-nums">{r.meta} 21:34</span>
        </div>
      {/each}
      <div class="px-3 py-1 text-chat-body">
        <span class="text-chat-amount tabular-nums">¥10,000</span>
        <span class="ml-2 font-bold">ねむいペンギン</span>
        <span class="ml-2">次の配信も楽しみにしてます</span>
      </div>
    </div>
  </section>

  <!-- 面と文字 -->
  <section class="mt-8">
    <h2 class="text-label text-fg-2">面の段階と文字の明るさ</h2>
    <div class="mt-3 grid grid-cols-5 border border-outline-variant">
      {#each SURFACES as s (s)}
        <div class="p-3" style="background: var(--sys-{s})">
          <p class="text-caption text-fg-3">{s}</p>
          <p class="text-body text-fg-1">fg-1 本文</p>
          <p class="text-body text-fg-2">fg-2 補足</p>
          <p class="text-body text-fg-3">fg-3 時刻</p>
        </div>
      {/each}
    </div>
  </section>

  <!-- 状態の色 -->
  <section class="mt-8">
    <h2 class="text-label text-fg-2">操作と状態</h2>
    <div class="mt-3 flex flex-wrap items-center gap-4 text-body">
      <span class="text-accent">accent 選択中</span>
      <span class="text-success">success 接続中</span>
      <span class="text-warning">warning 注意</span>
      <span class="text-error">error 切断</span>
      <span class="rounded-control bg-member px-2 py-0.5 text-label text-on-member">member メンバー</span>
      <button class="rounded-control border border-outline px-3 py-1 text-label hover:bg-fg-1/8 active:bg-fg-1/12">
        ボタン
      </button>
      <span class="rounded-control bg-surface-highest px-3 py-1 text-body text-fg-2 outline-2 outline-accent">
        フォーカス
      </span>
    </div>
  </section>

  <!-- 角丸と影 -->
  <section class="mt-8">
    <h2 class="text-label text-fg-2">角丸と影</h2>
    <div class="mt-3 flex items-start gap-6">
      <div class="rounded-control bg-surface-high px-3 py-2 text-caption">control 4px</div>
      <div class="rounded-popup bg-surface-high p-3 text-caption shadow-elevation-1">popup 8px + elevation-1</div>
      <div class="rounded-popup bg-surface-high p-3 text-caption shadow-elevation-2">dialog 8px + elevation-2</div>
      <div class="size-8 rounded-full bg-surface-highest"></div>
    </div>
  </section>

  <!-- ハイライト -->
  <section class="mt-8">
    <h2 class="text-label text-fg-2">ハイライト (優先順位 2〜4)</h2>
    <div class="mt-3 max-w-md bg-surface-low" style="font-size: 13px">
      <div class="bg-highlight/20 px-3 py-1 text-chat-body">
        <span class="rounded-control bg-highlight px-1 text-chat-meta font-bold text-surface">初見</span>
        <span class="ml-1 font-bold">{rows.first.name}</span>
        <span class="ml-2">{rows.first.body}</span>
      </div>
      <div class="px-3 py-1 text-chat-body">
        <span class="rounded-control border border-highlight px-1 text-chat-meta text-highlight">初見</span>
        <span class="ml-1 font-bold">{rows.newRepeat.name}</span>
        <span class="ml-2">{rows.newRepeat.body}</span>
      </div>
      <div class="px-3 py-1 text-chat-body">
        <span class="text-chat-meta text-highlight">●</span>
        <span class="ml-1 font-bold">{rows.returning.name}</span>
        <span class="ml-2">{rows.returning.body}</span>
      </div>
      <div class="px-3 py-1 text-chat-body">
        <span class="font-bold">{rows.normal.name}</span>
        <span class="ml-2">{rows.normal.body}</span>
      </div>
    </div>
  </section>
</main>
