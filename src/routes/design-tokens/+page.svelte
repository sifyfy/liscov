<script lang="ts">
  /**
   * デザイントークンの見本ページ (docs/design/tokens.md)
   *
   * 全トークンを並べ、撮影 (pnpm design:capture) してシフィさんが値を判断するための画面。
   * アプリのナビゲーションからは開けない。/design-tokens に直接移動して開く。
   * ここでは --sys-* と Tailwind に登録した名前だけを使う (生の色値・任意値を使わない)。
   * コメントの行は本物の ChatMessage を固定の架空データ (design-proto/data.ts の ROW_SAMPLES) で並べる。fontSize 13 は利用者の設定の既定の代わり。
   */
  import ChatMessage from '$lib/components/chat/ChatMessage.svelte';
  import FlowedSection from '$lib/components/chat/FlowedSection.svelte';
  import ThanksList from '$lib/components/chat/ThanksList.svelte';
  import StatusLine from '$lib/components/chat/StatusLine.svelte';
  import AuthIndicator from '$lib/components/auth/AuthIndicator.svelte';
  import type { AuthIndicatorState } from '$lib/types';
  import type { StatusAlert } from '$lib/utils/status-alerts';
  import { ROW_SAMPLES, feedOf, newcomersOf, originOf, supportersOf } from '../design-proto/data';

  // 流れたものの欄とお礼の一覧の見本 (試作の通常の場面のデータ)
  const feed = feedOf('normal');
  const supporters = supportersOf(feed);
  const flowedSupporters = [...supporters].reverse();
  const flowedNewcomers = newcomersOf(feed);
  const noInfo = () => undefined;

  // 状態の行の見本 (02_chat.md「状態 (異常のときだけ目立つ)」)。統計とリアクションは配信中のありがちな値
  const REACTIONS = [
    ['❤', 430],
    ['🎉', 418],
    ['😄', 94],
  ] as const;
  const STATUS_SAMPLES: readonly { label: string; alerts: readonly StatusAlert[]; error: string | null }[] = [
    { label: 'ふだん', alerts: [], error: null },
    { label: 'セッション切れ', alerts: ['session_expired'], error: null },
    { label: 'セッション切れと WebSocket の停止', alerts: ['session_expired', 'websocket_stopped'], error: null },
    { label: '接続のエラー', alerts: [], error: 'チャットの取得に15回続けて失敗したため切断しました' },
  ];
  const AUTH_STATES: readonly AuthIndicatorState[] = [
    'unauthenticated',
    'authenticated_valid',
    'authenticated_checking',
    'authenticated_invalid',
    'authenticated_error',
    'storage_error',
  ];

  const SAMPLE = '初見です！よろしくお願いします 0123456789 ¥10,000 さっきの謎解き、本棚の順番は左から赤・青・緑';

  const SURFACES = ['surface', 'surface-low', 'surface-mid', 'surface-high', 'surface-highest'] as const;

  const TYPE_SCALE = [
    { cls: 'text-caption', label: 'caption 12/16' },
    { cls: 'text-label', label: 'label 13/18 700' },
    { cls: 'text-body', label: 'body 14/20' },
    { cls: 'text-title', label: 'title 16/24 700' },
    { cls: 'text-display', label: 'display 20/28 700' },
  ] as const;
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

  <!-- コメントの行 (本物のコンポーネント。見た目の回帰テスト e2e/chat-row-visual.spec.ts が撮る) -->
  <section class="mt-8">
    <h2 class="text-label text-fg-2">コメントの行 (優先順位 1〜5・種類・相手の配信)</h2>
    <div data-testid="row-samples" class="mt-3 max-w-lg bg-surface-low py-1">
      {#each ROW_SAMPLES as message (message.id)}
        <ChatMessage {message} fontSize={13} showTimestamps origin={originOf(message)} />
      {/each}
    </div>
  </section>

  <!-- 流れたものの欄とお礼の一覧 (見た目の回帰テスト e2e/chat-row-visual.spec.ts が撮る) -->
  <section class="mt-8">
    <h2 class="text-label text-fg-2">流れたものの欄とお礼の一覧</h2>
    <div data-testid="flow-samples" class="mt-3 flex max-w-lg flex-col bg-surface">
      <ThanksList items={supporters} fontSize={13} showTimestamps {originOf} infoOf={noInfo} onClose={() => {}} />
      <FlowedSection label="流れた新規さん" items={flowedNewcomers} fontSize={13} showTimestamps {originOf} infoOf={noInfo} />
      <FlowedSection label="流れたスパチャ等" items={flowedSupporters} fontSize={13} showTimestamps {originOf} infoOf={noInfo} />
    </div>
  </section>

  <!-- 状態の行とログインの状態 (本物のコンポーネント) -->
  <section class="mt-8">
    <h2 class="text-label text-fg-2">状態の行 (異常のときだけ目立つ)</h2>
    <div data-testid="status-samples" class="mt-3 flex max-w-lg flex-col gap-3">
      {#each STATUS_SAMPLES as sample (sample.label)}
        <div>
          <p class="text-caption text-fg-3">{sample.label}</p>
          <div class="bg-surface">
            <StatusLine
              connected
              count={1820}
              perMinute={48}
              viewers={312}
              shownLabel={null}
              reactions={REACTIONS}
              reactionsLabel="リアクション"
              error={sample.error}
              onDismissError={() => {}}
              alerts={sample.alerts}
              onAlert={() => {}}
            />
          </div>
        </div>
      {/each}
    </div>
    <h2 class="mt-6 text-label text-fg-2">ログインの状態 (操作パネルの下段)</h2>
    <div data-testid="auth-samples" class="mt-3 flex max-w-lg flex-col items-start gap-2 bg-surface-low px-3 py-2 text-caption text-fg-2">
      {#each AUTH_STATES as state (state)}
        <AuthIndicator {state} onclick={() => {}} />
      {/each}
    </div>
  </section>
</main>
