<script lang="ts">
  import { viewerStore } from '$lib/stores';
  import type { ViewerWithCustomInfo } from '$lib/types';
  import DeleteConfirmDialog from './DeleteConfirmDialog.svelte';

  interface Props {
    viewer: ViewerWithCustomInfo;
    broadcasterId: string;
    onClose: () => void;
  }

  let { viewer, broadcasterId, onClose }: Props = $props();

  let reading = $state(viewer.reading || '');
  let notes = $state(viewer.notes || '');
  let tagsInput = $state(viewer.tags?.join(', ') || '');
  let isSaving = $state(false);
  let isDeleting = $state(false);
  let showDeleteConfirm = $state(false);
  let error = $state<string | null>(null);

  function parseTags(input: string): string[] {
    return input
      .split(',')
      .map(tag => tag.trim())
      .filter(tag => tag.length > 0);
  }

  async function handleSave() {
    isSaving = true;
    error = null;

    try {
      const tags = parseTags(tagsInput);
      await viewerStore.updateViewerInfo(
        viewer.id,
        reading || null,
        notes || null,
        viewer.custom_data ?? null,
        tags.length > 0 ? tags : null
      );
      onClose();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete() {
    isDeleting = true;
    error = null;

    try {
      await viewerStore.deleteViewer(viewer.id);
      showDeleteConfirm = false;
      onClose();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      isDeleting = false;
    }
  }

  function formatContribution(amount: number): string {
    if (amount === 0) return '-';
    return new Intl.NumberFormat('ja-JP', {
      style: 'currency',
      currency: 'JPY',
      maximumFractionDigits: 0
    }).format(amount);
  }
</script>

<!-- Modal backdrop -->
<div
  class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') onClose();
  }}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <!-- Modal content -->
  <div class="bg-[var(--bg-surface-2)] rounded-xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden border border-[var(--border-default)]">
    <!-- Header -->
    <div class="px-6 py-4 bg-[var(--bg-surface-3)] border-b border-[var(--border-default)]">
      <div class="flex items-center justify-between">
        <h2 class="text-xl font-semibold text-[var(--text-primary)]">視聴者情報の編集</h2>
        <button
          onclick={onClose}
          class="p-1 text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors"
          aria-label="Close"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Body -->
    <div class="p-6 space-y-6">
      <!-- Viewer info (read-only) -->
      <div class="space-y-3">
        <div>
          <span class="text-sm text-[var(--text-secondary)]">表示名</span>
          <p class="text-[var(--text-primary)] font-medium">{viewer.display_name}</p>
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <span class="text-sm text-[var(--text-secondary)]">コメント数</span>
            <p class="text-[var(--text-primary)]">{viewer.message_count.toLocaleString()}</p>
          </div>
          <div>
            <span class="text-sm text-[var(--text-secondary)]">貢献額</span>
            <p class="text-yellow-400">{formatContribution(viewer.total_contribution)}</p>
          </div>
        </div>
        {#if viewer.membership_level}
          <div>
            <span class="text-sm text-[var(--text-secondary)]">メンバーシップ</span>
            <p class="text-[var(--success)]">{viewer.membership_level}</p>
          </div>
        {/if}
      </div>

      <!-- Editable fields -->
      <div class="space-y-4">
        <div>
          <label for="reading" class="block text-sm text-[var(--text-secondary)] mb-1">
            読み仮名
          </label>
          <input
            id="reading"
            type="text"
            bind:value={reading}
            placeholder="TTS用の読み仮名を入力..."
            class="w-full px-4 py-2 rounded-lg bg-[var(--bg-surface-2)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50"
          />
          <p class="mt-1 text-xs text-[var(--text-muted)]">
            TTS読み上げ時に使用されます
          </p>
        </div>

        <div>
          <label for="notes" class="block text-sm text-[var(--text-secondary)] mb-1">
            メモ
          </label>
          <textarea
            id="notes"
            bind:value={notes}
            placeholder="視聴者に関するメモを追加..."
            rows="3"
            class="w-full px-4 py-2 rounded-lg bg-[var(--bg-surface-2)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50 resize-none"
          ></textarea>
        </div>

        <div>
          <label for="tags" class="block text-sm text-[var(--text-secondary)] mb-1">
            タグ
          </label>
          <input
            id="tags"
            type="text"
            bind:value={tagsInput}
            placeholder="タグ1, タグ2, タグ3..."
            class="w-full px-4 py-2 rounded-lg bg-[var(--bg-surface-2)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50"
          />
          <p class="mt-1 text-xs text-[var(--text-muted)]">
            カンマ区切りでタグを入力
          </p>
        </div>
      </div>

      {#if error}
        <p class="text-[var(--error)] text-sm">{error}</p>
      {/if}
    </div>

    <!-- Footer -->
    <div class="px-6 py-4 bg-[var(--bg-surface-3)] border-t border-[var(--border-default)] flex justify-between">
      <button
        onclick={() => showDeleteConfirm = true}
        disabled={isSaving || isDeleting}
        class="px-4 py-2 text-[var(--error)] hover:opacity-80 transition-colors disabled:opacity-50"
      >
        削除
      </button>
      <div class="flex gap-3">
        <button
          onclick={onClose}
          class="px-4 py-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors"
        >
          キャンセル
        </button>
        <button
          onclick={handleSave}
          disabled={isSaving}
          class="px-6 py-2 text-[var(--text-inverse)] font-semibold rounded-lg transition-colors disabled:opacity-50"
          style="background: var(--accent);"
        >
          {isSaving ? '保存中...' : '保存'}
        </button>
      </div>
    </div>
  </div>
</div>

<!-- Delete Confirmation Dialog -->
{#if showDeleteConfirm}
  <DeleteConfirmDialog
    title="カスタム情報の削除"
    message="この視聴者の読み仮名とメモが削除されます。視聴者プロフィールは保持されます。よろしいですか？"
    {isDeleting}
    onConfirm={handleDelete}
    onCancel={() => showDeleteConfirm = false}
  />
{/if}
