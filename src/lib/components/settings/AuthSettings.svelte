<script lang="ts">
  import { authStore } from '$lib/stores';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { normalizeError } from '$lib/tauri/errors';

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
      loginError = normalizeError(error).message;
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

<div class="flex flex-col gap-4 p-4 text-fg-1">
  <header class="flex flex-wrap items-center gap-x-3 gap-y-1">
    <h2 class="text-title">YouTube認証</h2>
    <span class="flex items-center gap-1.5 text-caption text-fg-2">
      <span
        class="size-1.5 rounded-full {authStore.isAuthenticated ? 'bg-success' : 'bg-fg-3'}"
        aria-hidden="true"
      ></span>
      {authStore.isAuthenticated ? '認証済み' : '未認証'}
    </span>
  </header>

  <section class="flex flex-col gap-3">
    <div class="flex flex-col gap-1">
      <h3 class="text-label">メンバー限定配信</h3>
      <p class="text-body text-fg-2">
        {authStore.isAuthenticated
          ? '認証済み - メンバー限定配信のチャットを取得できます'
          : 'メンバー限定配信を視聴するにはYouTubeへのログインが必要です'}
      </p>
    </div>

    {#if !authStore.isAuthenticated}
      <button
        type="button"
        onclick={handleLogin}
        disabled={isLoggingIn}
        class="self-start rounded-control bg-accent px-4 py-1.5 text-label text-surface hover:opacity-90 disabled:opacity-50"
      >
        {isLoggingIn ? 'ログイン中...' : 'YouTubeにログイン'}
      </button>
      <p class="text-caption text-fg-2">
        ボタンをクリックすると別ウィンドウでYouTubeのログイン画面が開きます。ログイン完了後、自動的に認証情報が保存されます。
      </p>
    {:else}
      <button
        type="button"
        onclick={handleLogout}
        class="self-start rounded-control border border-outline px-3 py-1 text-label text-error hover:bg-fg-1/8"
      >
        ログアウト
      </button>
    {/if}

    {#if loginError}
      <p role="alert" class="text-body text-error">{loginError}</p>
    {/if}
    {#if authStore.error}
      <p role="alert" class="text-body text-error">{authStore.error}</p>
    {/if}
  </section>

  <section class="flex flex-col gap-2 border-t border-outline-variant pt-4">
    <h3 class="text-label">ヘルプ</h3>
    <ul class="flex list-disc flex-col gap-1 pl-5 text-body text-fg-2">
      <li>「YouTubeにログイン」ボタンをクリックすると、アプリ内でYouTubeのログイン画面が開きます。</li>
      <li>Googleアカウントでログインすると、自動的に認証情報が保存されます。</li>
      <li>メンバー限定配信のチャットを取得するには、そのチャンネルのメンバーシップに加入している必要があります。</li>
    </ul>
  </section>
</div>
