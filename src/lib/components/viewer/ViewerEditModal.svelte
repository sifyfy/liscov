<script lang="ts">
  import { viewerStore } from '$lib/stores';
  import type { ViewerWithCustomInfo } from '$lib/types';

  interface Props {
    viewer: ViewerWithCustomInfo;
    broadcasterId: string;
    onClose: () => void;
  }

  let { viewer, broadcasterId, onClose }: Props = $props();

  let reading = $state(viewer.reading || '');
  let notes = $state(viewer.notes || '');
  let isSaving = $state(false);
  let error = $state<string | null>(null);

  async function handleSave() {
    isSaving = true;
    error = null;

    try {
      await viewerStore.updateViewerCustomInfo(
        broadcasterId,
        viewer.channel_id,
        reading || null,
        notes || null
      );
      onClose();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      isSaving = false;
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
  <div class="bg-gray-900 rounded-xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden">
    <!-- Header -->
    <div class="px-6 py-4 bg-white/5 border-b border-white/10">
      <div class="flex items-center justify-between">
        <h2 class="text-xl font-semibold text-white">Edit Viewer Info</h2>
        <button
          onclick={onClose}
          class="p-1 text-purple-300 hover:text-white transition-colors"
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
          <span class="text-sm text-purple-300">Display Name</span>
          <p class="text-white font-medium">{viewer.display_name}</p>
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <span class="text-sm text-purple-300">Messages</span>
            <p class="text-white">{viewer.message_count.toLocaleString()}</p>
          </div>
          <div>
            <span class="text-sm text-purple-300">Contribution</span>
            <p class="text-yellow-300">{formatContribution(viewer.total_contribution)}</p>
          </div>
        </div>
        {#if viewer.membership_level}
          <div>
            <span class="text-sm text-purple-300">Membership</span>
            <p class="text-green-300">{viewer.membership_level}</p>
          </div>
        {/if}
      </div>

      <!-- Editable fields -->
      <div class="space-y-4">
        <div>
          <label for="reading" class="block text-sm text-purple-300 mb-1">
            Reading (Furigana)
          </label>
          <input
            id="reading"
            type="text"
            bind:value={reading}
            placeholder="Enter reading for TTS..."
            class="w-full px-4 py-2 rounded-lg bg-white/10 text-white placeholder-purple-400 border border-white/20 focus:outline-none focus:ring-2 focus:ring-purple-400"
          />
          <p class="mt-1 text-xs text-purple-400">
            Used for TTS pronunciation
          </p>
        </div>

        <div>
          <label for="notes" class="block text-sm text-purple-300 mb-1">
            Notes
          </label>
          <textarea
            id="notes"
            bind:value={notes}
            placeholder="Add notes about this viewer..."
            rows="3"
            class="w-full px-4 py-2 rounded-lg bg-white/10 text-white placeholder-purple-400 border border-white/20 focus:outline-none focus:ring-2 focus:ring-purple-400 resize-none"
          ></textarea>
        </div>
      </div>

      {#if error}
        <p class="text-red-400 text-sm">{error}</p>
      {/if}
    </div>

    <!-- Footer -->
    <div class="px-6 py-4 bg-white/5 border-t border-white/10 flex justify-end gap-3">
      <button
        onclick={onClose}
        class="px-4 py-2 text-purple-200 hover:text-white transition-colors"
      >
        Cancel
      </button>
      <button
        onclick={handleSave}
        disabled={isSaving}
        class="px-6 py-2 bg-purple-500 text-white font-semibold rounded-lg hover:bg-purple-400 transition-colors disabled:opacity-50"
      >
        {isSaving ? 'Saving...' : 'Save'}
      </button>
    </div>
  </div>
</div>
