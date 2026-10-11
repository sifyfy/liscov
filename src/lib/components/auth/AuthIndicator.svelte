<script lang="ts">
  /**
   * ログインの状態 (docs/specs/01_auth.md「AuthIndicator.svelte」)
   *
   * 操作パネルの下段に、WebSocket の状態と同じ「小さい点 + 文字」で出す。文字は全文を出し、ヒントに頼らない。
   * 押すと設定のログインを開く
   */
  import type { AuthIndicatorState } from '$lib/types';

  interface Props {
    state: AuthIndicatorState;
    onclick: () => void;
  }
  let { state, onclick }: Props = $props();

  const STATES = {
    unauthenticated: { dot: 'bg-fg-3', label: '未ログイン', error: false },
    authenticated_valid: { dot: 'bg-success', label: 'ログイン中', error: false },
    authenticated_checking: { dot: 'bg-fg-3', label: 'ログイン中: 検証中', error: false },
    authenticated_invalid: { dot: 'bg-error', label: 'ログイン: セッション切れ。再ログインが必要です', error: true },
    authenticated_error: { dot: 'bg-warning', label: 'ログイン中: 検証失敗 (ネットワークエラー)', error: false },
    storage_error: { dot: 'bg-error', label: 'ストレージエラー。設定を確認してください', error: true },
  } as const satisfies Record<AuthIndicatorState, { dot: string; label: string; error: boolean }>;

  let view = $derived(STATES[state]);
</script>

<button
  type="button"
  data-testid="auth-indicator"
  {onclick}
  class="-mx-1 flex items-center gap-1.5 rounded-control px-1 text-left hover:bg-fg-1/8 {view.error ? 'text-error' : ''}"
>
  <span data-testid="auth-dot" class="size-1.5 shrink-0 rounded-full {view.dot}" aria-hidden="true"></span>
  {view.label}
</button>
