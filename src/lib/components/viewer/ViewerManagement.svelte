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

<div class="flex flex-col h-full bg-[var(--bg-surface-2)] rounded-xl overflow-hidden border border-[var(--border-default)]">
  <!-- Header -->
  <div class="px-6 py-4 bg-[var(--bg-surface-3)] border-b border-[var(--border-default)]">
    <div class="flex items-center justify-between mb-3">
      <div>
        <h2 class="text-xl font-semibold text-[var(--text-primary)]" style="font-family: var(--font-heading);">視聴者管理</h2>
        <p class="text-sm text-[var(--text-secondary)] mt-1">
          視聴者プロフィールとカスタム情報を管理
        </p>
      </div>
      {#if selectedBroadcasterId}
        <button
          onclick={() => showDeleteBroadcasterConfirm = true}
          class="px-3 py-1.5 text-sm text-[var(--error)] hover:opacity-80 rounded transition-colors border border-[var(--border-default)]"
        >
          配信者を削除
        </button>
      {/if}
    </div>
    <BroadcasterSelector
      selected={selectedBroadcasterId}
      onSelect={handleBroadcasterSelect}
    />
    <!-- 配信者削除エラー表示 -->
    {#if deleteError}
      <p class="mt-2 text-sm text-[var(--error)]">{deleteError}</p>
    {/if}
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-hidden">
    {#if selectedBroadcasterId}
      <ViewerList broadcasterId={selectedBroadcasterId} />
    {:else}
      <div class="flex items-center justify-center h-full">
        <p class="text-[var(--text-muted)]">配信者を選択してください</p>
      </div>
    {/if}
  </div>
</div>

<!-- Edit Modal -->
{#if showEditModal && viewerStore.selectedViewer && selectedBroadcasterId}
  <ViewerEditModal
    viewer={viewerStore.selectedViewer}
    onClose={handleCloseModal}
  />
{/if}

<!-- Delete Broadcaster Confirmation -->
{#if showDeleteBroadcasterConfirm}
  <DeleteConfirmDialog
    title="配信者データの削除"
    message="この配信者のすべてのカスタム視聴者情報が削除されます。配信者間で共有される視聴者プロフィールは保持されます。よろしいですか？"
    isDeleting={isDeletingBroadcaster}
    onConfirm={handleDeleteBroadcaster}
    onCancel={() => showDeleteBroadcasterConfirm = false}
  />
{/if}
