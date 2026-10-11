<script lang="ts">
  import { authStore } from '$lib/stores';
  import Icon from '$lib/components/ui/Icon.svelte';

  // Props
  interface Props {
    open: boolean;
    onClose: () => void;
    onOpenSettings: () => void;
  }
  let { open, onClose, onOpenSettings }: Props = $props();

  async function handleUseFallback() {
    await authStore.useFallbackStorage();
    onClose();
  }

  function handleOpenSettings() {
    onOpenSettings();
    onClose();
  }

  function handleIgnore() {
    onClose();
  }
</script>

{#if open}
  <!-- 外側を押す・Esc で閉じる (「無視」と同じ) -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-surface/80 p-4"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
    onkeydown={(e) => e.key === 'Escape' && onClose()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="dialog-title"
    tabindex="-1"
  >
    <div class="flex w-full max-w-md flex-col gap-3 rounded-popup border border-error bg-surface-high p-4 text-fg-1 shadow-elevation-2">
      <h2 id="dialog-title" class="flex items-center gap-2 text-title">
        <Icon name="warning" size={20} class="shrink-0 text-error" />
        セキュアストレージが利用できません
      </h2>
      <p class="text-body">
        Windows資格情報マネージャーにアクセスできません。 代わりにファイル保存（credentials.toml）を使用しますか？
      </p>
      <p class="text-body text-warning">※ファイル保存はセキュリティが低下します</p>
      <!-- 並びは 01_auth.md「ストレージ障害通知ダイアログ」の順 -->
      <div class="flex flex-wrap gap-2">
        <button
          type="button"
          onclick={handleUseFallback}
          class="rounded-control bg-accent px-3 py-1 text-label text-surface hover:opacity-90"
        >
          ファイル保存を使用
        </button>
        <button
          type="button"
          onclick={handleOpenSettings}
          class="rounded-control border border-outline px-3 py-1 text-label text-fg-2 hover:bg-fg-1/8"
        >
          設定を開く
        </button>
        <button
          type="button"
          onclick={handleIgnore}
          class="rounded-control px-3 py-1 text-label text-fg-2 hover:bg-fg-1/8"
        >
          無視
        </button>
      </div>
    </div>
  </div>
{/if}
