<script lang="ts">
  import { authStore } from '$lib/stores';

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
  <!-- Backdrop -->
  <div
    class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center"
    onclick={onClose}
    role="button"
    tabindex="-1"
    onkeydown={(e) => e.key === 'Escape' && onClose()}
  >
    <!-- Dialog -->
    <div
      class="bg-[var(--bg-surface-2)] rounded-lg shadow-xl max-w-md w-full mx-4 p-6"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
    >
      <!-- Header -->
      <div class="flex items-center gap-3 mb-4">
        <div class="w-10 h-10 rounded-full bg-[var(--error)]/20 flex items-center justify-center">
          <svg class="w-6 h-6 text-[var(--error)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        </div>
        <h2 id="dialog-title" class="text-lg font-semibold text-[var(--text-primary)]">
          セキュアストレージが利用できません
        </h2>
      </div>

      <!-- Content -->
      <div class="space-y-3 mb-6">
        <p class="text-[var(--text-secondary)]">
          Windows資格情報マネージャーにアクセスできません。
          代わりにファイル保存（credentials.toml）を使用しますか？
        </p>
        <p class="text-sm text-[var(--warning)] bg-[var(--warning-subtle)] px-3 py-2 rounded-md">
          ※ファイル保存はセキュリティが低下します
        </p>
      </div>

      <!-- Actions -->
      <div class="flex gap-3">
        <button
          onclick={handleUseFallback}
          class="flex-1 px-4 py-2 bg-[var(--accent)] text-[var(--text-inverse)] rounded-lg hover:bg-[var(--accent-hover)] transition-colors font-medium"
        >
          ファイル保存を使用
        </button>
        <button
          onclick={handleOpenSettings}
          class="px-4 py-2 bg-[var(--bg-surface-3)] text-[var(--text-secondary)] rounded-lg hover:bg-[var(--bg-elevated)] transition-colors"
        >
          設定を開く
        </button>
        <button
          onclick={handleIgnore}
          class="px-4 py-2 text-[var(--text-muted)] hover:text-[var(--text-secondary)] transition-colors"
        >
          無視
        </button>
      </div>
    </div>
  </div>
{/if}
