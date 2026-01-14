<script lang="ts">
  import { authStore } from '$lib/stores';
  import type { AuthIndicatorState } from '$lib/types';

  // Props
  interface Props {
    onclick?: () => void;
  }
  let { onclick }: Props = $props();

  // Get indicator state from store
  let state = $derived(authStore.indicatorState);

  // State configurations
  const stateConfig: Record<AuthIndicatorState, {
    color: string;
    bgColor: string;
    icon: string;
    tooltip: string;
    spin?: boolean;
  }> = {
    unauthenticated: {
      color: 'text-gray-400',
      bgColor: 'bg-gray-400/20',
      icon: 'lock',
      tooltip: '未ログイン'
    },
    authenticated_valid: {
      color: 'text-green-400',
      bgColor: 'bg-green-400/20',
      icon: 'unlock',
      tooltip: 'ログイン中: 有効'
    },
    authenticated_checking: {
      color: 'text-yellow-400',
      bgColor: 'bg-yellow-400/20',
      icon: 'spinner',
      tooltip: 'ログイン中: 検証中...',
      spin: true
    },
    authenticated_invalid: {
      color: 'text-red-400',
      bgColor: 'bg-red-400/20',
      icon: 'warning',
      tooltip: 'ログイン中: セッション切れ - 再ログインが必要'
    },
    authenticated_error: {
      color: 'text-orange-400',
      bgColor: 'bg-orange-400/20',
      icon: 'question',
      tooltip: 'ログイン中: 検証失敗（ネットワークエラー）'
    },
    storage_error: {
      color: 'text-red-400',
      bgColor: 'bg-red-400/20',
      icon: 'exclamation',
      tooltip: 'ストレージエラー - 設定を確認してください'
    }
  };

  let config = $derived(stateConfig[state]);
</script>

<button
  data-testid="auth-indicator"
  class="flex items-center gap-2 px-3 py-1.5 rounded-lg transition-colors hover:bg-white/10 {config.bgColor}"
  title={config.tooltip}
  onclick={onclick}
>
  <span class="relative {config.color}">
    {#if config.icon === 'lock'}
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
      </svg>
    {:else if config.icon === 'unlock'}
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 11V7a4 4 0 118 0m-4 8v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2z" />
      </svg>
    {:else if config.icon === 'spinner'}
      <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
      </svg>
    {:else if config.icon === 'warning'}
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
      </svg>
    {:else if config.icon === 'question'}
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
    {:else if config.icon === 'exclamation'}
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
    {/if}
  </span>
  <span class="text-sm text-white/80">
    {#if state === 'unauthenticated'}
      未ログイン
    {:else if state === 'authenticated_valid'}
      認証済み
    {:else if state === 'authenticated_checking'}
      検証中
    {:else if state === 'authenticated_invalid'}
      セッション切れ
    {:else if state === 'authenticated_error'}
      検証失敗
    {:else if state === 'storage_error'}
      エラー
    {/if}
  </span>
</button>
