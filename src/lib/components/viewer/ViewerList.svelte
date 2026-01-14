<script lang="ts">
  import { viewerStore } from '$lib/stores';
  import type { ViewerWithCustomInfo } from '$lib/types';

  interface Props {
    broadcasterId: string;
  }

  let { broadcasterId }: Props = $props();

  let localSearchQuery = $state('');

  $effect(() => {
    if (broadcasterId) {
      viewerStore.loadViewers(broadcasterId);
    }
  });

  function handleSearch() {
    viewerStore.searchViewers(broadcasterId, localSearchQuery);
  }

  function formatContribution(amount: number): string {
    if (amount === 0) return '-';
    return new Intl.NumberFormat('ja-JP', {
      style: 'currency',
      currency: 'JPY',
      maximumFractionDigits: 0
    }).format(amount);
  }

  function formatDate(dateStr: string): string {
    try {
      return new Date(dateStr).toLocaleDateString('ja-JP');
    } catch {
      return dateStr;
    }
  }
</script>

<div class="flex flex-col h-full">
  <!-- Search bar -->
  <div class="p-4 bg-white/5 border-b border-white/10">
    <form
      class="flex gap-2"
      onsubmit={(e) => {
        e.preventDefault();
        handleSearch();
      }}
    >
      <input
        type="text"
        bind:value={localSearchQuery}
        placeholder="Search by name, reading, or notes..."
        class="flex-1 px-3 py-2 rounded-lg bg-white/10 text-white placeholder-purple-300 border border-white/20 focus:outline-none focus:ring-2 focus:ring-purple-400"
      />
      <button
        type="submit"
        class="px-4 py-2 bg-purple-500 text-white rounded-lg hover:bg-purple-400 transition-colors"
      >
        Search
      </button>
    </form>
  </div>

  <!-- Viewer list -->
  <div class="flex-1 overflow-y-auto">
    {#if viewerStore.isLoading}
      <div class="flex items-center justify-center h-32">
        <p class="text-purple-300">Loading...</p>
      </div>
    {:else if viewerStore.viewers.length === 0}
      <div class="flex items-center justify-center h-32">
        <p class="text-purple-300">No viewers found</p>
      </div>
    {:else}
      <table class="w-full">
        <thead class="bg-white/5 sticky top-0">
          <tr class="text-left text-purple-300 text-sm">
            <th class="px-4 py-3">Name</th>
            <th class="px-4 py-3">Reading</th>
            <th class="px-4 py-3">Messages</th>
            <th class="px-4 py-3">Contribution</th>
            <th class="px-4 py-3">Last seen</th>
          </tr>
        </thead>
        <tbody>
          {#each viewerStore.viewers as viewer (viewer.channel_id)}
            <tr
              class="border-b border-white/5 hover:bg-white/5 cursor-pointer transition-colors {viewerStore.selectedViewer?.channel_id === viewer.channel_id ? 'bg-purple-500/20' : ''}"
              onclick={() => viewerStore.selectViewer(viewer)}
            >
              <td class="px-4 py-3">
                <div class="flex items-center gap-2">
                  <span class="text-white font-medium">{viewer.display_name}</span>
                  {#if viewer.membership_level}
                    <span class="px-1.5 py-0.5 text-xs bg-green-500/50 text-green-100 rounded">
                      Member
                    </span>
                  {/if}
                </div>
              </td>
              <td class="px-4 py-3 text-purple-200">
                {viewer.reading || '-'}
              </td>
              <td class="px-4 py-3 text-purple-200">
                {viewer.message_count.toLocaleString()}
              </td>
              <td class="px-4 py-3 text-yellow-300">
                {formatContribution(viewer.total_contribution)}
              </td>
              <td class="px-4 py-3 text-purple-300 text-sm">
                {formatDate(viewer.last_seen)}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>

  <!-- Pagination -->
  <div class="flex items-center justify-between px-4 py-3 bg-white/5 border-t border-white/10">
    <button
      onclick={() => viewerStore.prevPage()}
      disabled={viewerStore.currentPage === 0}
      class="px-4 py-2 text-sm text-purple-200 hover:text-white hover:bg-white/10 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    >
      Previous
    </button>
    <span class="text-purple-300 text-sm">
      Page {viewerStore.currentPage + 1}
    </span>
    <button
      onclick={() => viewerStore.nextPage()}
      disabled={viewerStore.viewers.length < viewerStore.pageSize}
      class="px-4 py-2 text-sm text-purple-200 hover:text-white hover:bg-white/10 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    >
      Next
    </button>
  </div>
</div>
