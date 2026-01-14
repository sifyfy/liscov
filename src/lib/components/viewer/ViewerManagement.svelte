<script lang="ts">
  import { viewerStore } from '$lib/stores';
  import ViewerList from './ViewerList.svelte';
  import ViewerEditModal from './ViewerEditModal.svelte';

  interface Props {
    broadcasterId: string;
  }

  let { broadcasterId }: Props = $props();

  let showEditModal = $state(false);

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
</script>

<div class="flex flex-col h-full bg-gray-900 rounded-xl overflow-hidden">
  <!-- Header -->
  <div class="px-6 py-4 bg-white/5 border-b border-white/10">
    <h2 class="text-xl font-semibold text-white">Viewer Management</h2>
    <p class="text-sm text-purple-300 mt-1">
      Manage viewer profiles and custom information
    </p>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-hidden">
    <ViewerList {broadcasterId} />
  </div>
</div>

<!-- Edit Modal -->
{#if showEditModal && viewerStore.selectedViewer}
  <ViewerEditModal
    viewer={viewerStore.selectedViewer}
    {broadcasterId}
    onClose={handleCloseModal}
  />
{/if}
