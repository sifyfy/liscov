<script lang="ts">
  import { authStore } from '$lib/stores';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Icon from '$lib/components/ui/Icon.svelte';

  let isLoggingIn = $state(false);
  let loginError = $state<string | null>(null);

  onMount(() => {
    authStore.refreshStatus();
  });

  async function handleLogin() {
    isLoggingIn = true;
    loginError = null;
    try {
      await invoke('auth_open_window');
      await authStore.refreshStatus();
      // Check session validity after login to update indicator state
      if (authStore.isAuthenticated) {
        authStore.checkSessionValidity();
      }
    } catch (error) {
      console.error('Login failed:', error);
      loginError = error instanceof Error ? error.message : String(error);
    } finally {
      isLoggingIn = false;
    }
  }

  async function handleLogout() {
    if (confirm('ログアウトしてよろしいですか？')) {
      try {
        // Delete saved credentials
        // Note: WebView cookies cannot be cleared in Tauri v2 (auth_clear_webview is deprecated)
        await authStore.deleteCredentials();
      } catch (error) {
        console.error('Logout failed:', error);
      }
    }
  }
</script>

<div class="p-6 space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold text-[var(--text-primary)]" style="font-family: var(--font-heading);">YouTube認証</h2>
    <div class="flex items-center gap-2">
      {#if authStore.isAuthenticated}
        <span class="px-2 py-1 text-xs bg-[var(--success-subtle)] text-[var(--success)] rounded border border-[var(--border-default)]">認証済み</span>
      {:else}
        <span class="px-2 py-1 text-xs bg-[var(--warning-subtle)] text-[var(--warning)] rounded border border-[var(--border-default)]">未認証</span>
      {/if}
    </div>
  </div>

  <!-- Main Authentication Section -->
  <div class="p-4 bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)] space-y-4">
    <div class="flex items-center gap-3">
      <Icon name={authStore.isAuthenticated ? 'unlock' : 'lock'} size={24} class="text-[var(--accent)]" />
      <div>
        <h3 class="text-[var(--text-primary)] font-medium">メンバー限定配信</h3>
        <p class="text-sm text-[var(--text-muted)]">
          {authStore.isAuthenticated
            ? '認証済み - メンバー限定配信のチャットを取得できます'
            : 'メンバー限定配信を視聴するにはYouTubeへのログインが必要です'}
        </p>
      </div>
    </div>

    {#if !authStore.isAuthenticated}
      <div class="space-y-4">
        <button
          onclick={handleLogin}
          disabled={isLoggingIn}
          class="w-full px-4 py-3 text-[var(--text-inverse)] rounded-lg transition-colors disabled:opacity-50 font-medium"
          style="background: var(--accent);"
        >
          {#if isLoggingIn}
            <span class="flex items-center justify-center gap-2">
              <svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              ログイン中...
            </span>
          {:else}
            YouTubeにログイン
          {/if}
        </button>

        <div class="p-3 bg-[var(--info-subtle)] border border-[var(--border-default)] rounded-lg text-sm text-[var(--info)]">
          <p>ボタンをクリックすると別ウィンドウでYouTubeのログイン画面が開きます。ログイン完了後、自動的に認証情報が保存されます。</p>
        </div>
      </div>
    {:else}
      <button
        onclick={handleLogout}
        class="px-4 py-2 bg-[var(--error-subtle)] text-[var(--error)] rounded-lg border border-[var(--border-default)] hover:opacity-80 transition-colors"
      >
        ログアウト
      </button>
    {/if}
  </div>

  <!-- Error Message -->
  {#if loginError}
    <div class="p-3 bg-[var(--error-subtle)] rounded-lg border border-[var(--border-default)]">
      <p class="text-[var(--error)] text-sm">{loginError}</p>
    </div>
  {/if}

  {#if authStore.error}
    <div class="p-3 bg-[var(--error-subtle)] rounded-lg border border-[var(--border-default)]">
      <p class="text-[var(--error)] text-sm">{authStore.error}</p>
    </div>
  {/if}

  <!-- Help Section -->
  <div class="p-4 bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)] space-y-2">
    <h4 class="text-[var(--text-primary)] font-medium text-sm">ヘルプ</h4>
    <div class="text-xs text-[var(--text-secondary)] space-y-1">
      <p>「YouTubeにログイン」ボタンをクリックすると、アプリ内でYouTubeのログイン画面が開きます。</p>
      <p>Googleアカウントでログインすると、自動的に認証情報が保存されます。</p>
      <p>メンバー限定配信のチャットを取得するには、そのチャンネルのメンバーシップに加入している必要があります。</p>
    </div>
  </div>
</div>
