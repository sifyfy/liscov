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
        broadcasterId,
        viewer.channel_id,
        reading || null,
        notes || null,
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
      await viewerStore.deleteViewerCustomInfo(broadcasterId, viewer.channel_id);
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
  <div class="bg-[var(--bg-white)] rounded-xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden border border-[var(--border-light)]">
    <!-- Header -->
    <div class="px-6 py-4 bg-[var(--bg-light)] border-b border-[var(--border-light)]">
      <div class="flex items-center justify-between">
        <h2 class="text-xl font-semibold text-[var(--text-primary)]">Edit Viewer Info</h2>
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
          <span class="text-sm text-[var(--text-secondary)]">Display Name</span>
          <p class="text-[var(--text-primary)] font-medium">{viewer.display_name}</p>
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <span class="text-sm text-[var(--text-secondary)]">Messages</span>
            <p class="text-[var(--text-primary)]">{viewer.message_count.toLocaleString()}</p>
          </div>
          <div>
            <span class="text-sm text-[var(--text-secondary)]">Contribution</span>
            <p class="text-yellow-600">{formatContribution(viewer.total_contribution)}</p>
          </div>
        </div>
        {#if viewer.membership_level}
          <div>
            <span class="text-sm text-[var(--text-secondary)]">Membership</span>
            <p class="text-green-600">{viewer.membership_level}</p>
          </div>
        {/if}
      </div>

      <!-- Editable fields -->
      <div class="space-y-4">
        <div>
          <label for="reading" class="block text-sm text-[var(--text-secondary)] mb-1">
            Reading (Furigana)
          </label>
          <input
            id="reading"
            type="text"
            bind:value={reading}
            placeholder="Enter reading for TTS..."
            class="w-full px-4 py-2 rounded-lg bg-[var(--bg-white)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-light)] focus:outline-none focus:ring-2 focus:ring-[var(--primary-start)]/50"
          />
          <p class="mt-1 text-xs text-[var(--text-muted)]">
            Used for TTS pronunciation
          </p>
        </div>

        <div>
          <label for="notes" class="block text-sm text-[var(--text-secondary)] mb-1">
            Notes
          </label>
          <textarea
            id="notes"
            bind:value={notes}
            placeholder="Add notes about this viewer..."
            rows="3"
            class="w-full px-4 py-2 rounded-lg bg-[var(--bg-white)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-light)] focus:outline-none focus:ring-2 focus:ring-[var(--primary-start)]/50 resize-none"
          ></textarea>
        </div>

        <div>
          <label for="tags" class="block text-sm text-[var(--text-secondary)] mb-1">
            Tags
          </label>
          <input
            id="tags"
            type="text"
            bind:value={tagsInput}
            placeholder="tag1, tag2, tag3..."
            class="w-full px-4 py-2 rounded-lg bg-[var(--bg-white)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-light)] focus:outline-none focus:ring-2 focus:ring-[var(--primary-start)]/50"
          />
          <p class="mt-1 text-xs text-[var(--text-muted)]">
            Comma-separated tags
          </p>
        </div>
      </div>

      {#if error}
        <p class="text-red-500 text-sm">{error}</p>
      {/if}
    </div>

    <!-- Footer -->
    <div class="px-6 py-4 bg-[var(--bg-light)] border-t border-[var(--border-light)] flex justify-between">
      <button
        onclick={() => showDeleteConfirm = true}
        disabled={isSaving || isDeleting}
        class="px-4 py-2 text-red-500 hover:text-red-600 transition-colors disabled:opacity-50"
      >
        Delete
      </button>
      <div class="flex gap-3">
        <button
          onclick={onClose}
          class="px-4 py-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors"
        >
          Cancel
        </button>
        <button
          onclick={handleSave}
          disabled={isSaving}
          class="px-6 py-2 text-white font-semibold rounded-lg transition-colors disabled:opacity-50"
          style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);"
        >
          {isSaving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  </div>
</div>

<!-- Delete Confirmation Dialog -->
{#if showDeleteConfirm}
  <DeleteConfirmDialog
    title="Delete Custom Info"
    message="This will remove the reading and notes for this viewer. The viewer profile will be kept. Are you sure?"
    {isDeleting}
    onConfirm={handleDelete}
    onCancel={() => showDeleteConfirm = false}
  />
{/if}
