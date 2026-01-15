<script lang="ts">
  import { viewerStore } from '$lib/stores';
  import ViewerList from './ViewerList.svelte';
  import ViewerEditModal from './ViewerEditModal.svelte';
  import BroadcasterSelector from './BroadcasterSelector.svelte';
  import DeleteConfirmDialog from './DeleteConfirmDialog.svelte';

  interface Props {
    broadcasterId?: string;
  }

  let { broadcasterId: initialBroadcasterId }: Props = $props();

  let selectedBroadcasterId = $state<string | null>(initialBroadcasterId || null);
  let showEditModal = $state(false);
  let showDeleteBroadcasterConfirm = $state(false);
  let isDeletingBroadcaster = $state(false);
  let deleteError = $state<string | null>(null);

  // Watch for viewer selection
  $effect(() => {
    if (viewerStore.selectedViewer) {
      showEditModal = true;
    }
  });

  function handleCloseModal() {
    showEditModal = false;
    viewerStore.clearSelection();
  }

  function handleBroadcasterSelect(broadcasterId: string | null) {
    selectedBroadcasterId = broadcasterId;
  }

  async function handleDeleteBroadcaster() {
    if (!selectedBroadcasterId) return;

    isDeletingBroadcaster = true;
    deleteError = null;

    try {
      await viewerStore.deleteBroadcaster(selectedBroadcasterId);
      selectedBroadcasterId = null;
      showDeleteBroadcasterConfirm = false;
    } catch (e) {
      deleteError = e instanceof Error ? e.message : String(e);
    } finally {
      isDeletingBroadcaster = false;
    }
  }
</script>

<div class="flex flex-col h-full bg-[var(--bg-white)] rounded-xl overflow-hidden border border-[var(--border-light)] shadow-sm">
  <!-- Header -->
  <div class="px-6 py-4 bg-[var(--bg-light)] border-b border-[var(--border-light)]">
    <div class="flex items-center justify-between mb-3">
      <div>
        <h2 class="text-xl font-semibold text-[var(--text-primary)]">Viewer Management</h2>
        <p class="text-sm text-[var(--text-secondary)] mt-1">
          Manage viewer profiles and custom information
        </p>
      </div>
      {#if selectedBroadcasterId}
        <button
          onclick={() => showDeleteBroadcasterConfirm = true}
          class="px-3 py-1.5 text-sm text-red-500 hover:text-red-600 hover:bg-red-50 rounded transition-colors border border-red-200"
        >
          Delete Broadcaster
        </button>
      {/if}
    </div>
    <BroadcasterSelector
      selected={selectedBroadcasterId}
      onSelect={handleBroadcasterSelect}
    />
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-hidden">
    {#if selectedBroadcasterId}
      <ViewerList broadcasterId={selectedBroadcasterId} />
    {:else}
      <div class="flex items-center justify-center h-full">
        <p class="text-[var(--text-muted)]">Select a broadcaster to view viewers</p>
      </div>
    {/if}
  </div>
</div>

<!-- Edit Modal -->
{#if showEditModal && viewerStore.selectedViewer && selectedBroadcasterId}
  <ViewerEditModal
    viewer={viewerStore.selectedViewer}
    broadcasterId={selectedBroadcasterId}
    onClose={handleCloseModal}
  />
{/if}

<!-- Delete Broadcaster Confirmation -->
{#if showDeleteBroadcasterConfirm}
  <DeleteConfirmDialog
    title="Delete Broadcaster Data"
    message="This will delete all custom viewer information for this broadcaster. Viewer profiles shared across broadcasters will be kept. Are you sure?"
    isDeleting={isDeletingBroadcaster}
    onConfirm={handleDeleteBroadcaster}
    onCancel={() => showDeleteBroadcasterConfirm = false}
  />
{/if}
