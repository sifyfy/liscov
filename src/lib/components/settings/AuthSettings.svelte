<script lang="ts">
  import { authStore } from '$lib/stores';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

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
      // Refresh status after successful login
      await authStore.refreshStatus();
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
    <h2 class="text-xl font-semibold text-[var(--text-primary)]">YouTube認証</h2>
    <div class="flex items-center gap-2">
      {#if authStore.isAuthenticated}
        <span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded border border-green-200">認証済み</span>
      {:else}
        <span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-700 rounded border border-yellow-200">未認証</span>
      {/if}
    </div>
  </div>

  <!-- Main Authentication Section -->
  <div class="p-4 bg-[var(--bg-white)] rounded-lg border border-[var(--border-light)] shadow-sm space-y-4">
    <div class="flex items-center gap-3">
      <span class="text-2xl">{authStore.isAuthenticated ? '🔓' : '🔒'}</span>
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
          class="w-full px-4 py-3 text-white rounded-lg transition-colors disabled:opacity-50 font-medium"
          style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);"
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

        <div class="p-3 bg-blue-50 border border-blue-200 rounded-lg text-sm text-blue-800">
          <p>ボタンをクリックすると別ウィンドウでYouTubeのログイン画面が開きます。ログイン完了後、自動的に認証情報が保存されます。</p>
        </div>
      </div>
    {:else}
      <button
        onclick={handleLogout}
        class="px-4 py-2 bg-red-50 text-red-600 rounded-lg border border-red-200 hover:bg-red-100 transition-colors"
      >
        ログアウト
      </button>
    {/if}
  </div>

  <!-- Error Message -->
  {#if loginError}
    <div class="p-3 bg-red-50 rounded-lg border border-red-200">
      <p class="text-red-600 text-sm">{loginError}</p>
    </div>
  {/if}

  {#if authStore.error}
    <div class="p-3 bg-red-50 rounded-lg border border-red-200">
      <p class="text-red-600 text-sm">{authStore.error}</p>
    </div>
  {/if}

  <!-- Help Section -->
  <div class="p-4 bg-[var(--bg-light)] rounded-lg border border-[var(--border-light)] space-y-2">
    <h4 class="text-[var(--text-primary)] font-medium text-sm">ヘルプ</h4>
    <div class="text-xs text-[var(--text-secondary)] space-y-1">
      <p>「YouTubeにログイン」ボタンをクリックすると、アプリ内でYouTubeのログイン画面が開きます。</p>
      <p>Googleアカウントでログインすると、自動的に認証情報が保存されます。</p>
      <p>メンバー限定配信のチャットを取得するには、そのチャンネルのメンバーシップに加入している必要があります。</p>
    </div>
  </div>
</div>
