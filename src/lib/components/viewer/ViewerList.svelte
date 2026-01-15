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

  function handleRefresh() {
    viewerStore.loadViewers(broadcasterId);
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
  <div class="p-4 bg-[var(--bg-light)] border-b border-[var(--border-light)]">
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
        class="flex-1 px-3 py-2 rounded-lg bg-[var(--bg-white)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-light)] focus:outline-none focus:ring-2 focus:ring-[var(--primary-start)]/50"
      />
      <button
        type="submit"
        class="px-4 py-2 text-white rounded-lg transition-colors"
        style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);"
      >
        Search
      </button>
      <button
        type="button"
        onclick={handleRefresh}
        class="px-3 py-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-light)] rounded-lg transition-colors border border-[var(--border-light)]"
        title="Refresh"
        aria-label="Refresh viewer list"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
      </button>
    </form>
  </div>

  <!-- Viewer list -->
  <div class="flex-1 overflow-y-auto">
    {#if viewerStore.isLoading}
      <div class="flex items-center justify-center h-32">
        <p class="text-[var(--text-muted)]">Loading...</p>
      </div>
    {:else if viewerStore.viewers.length === 0}
      <div class="flex items-center justify-center h-32">
        <p class="text-[var(--text-muted)]">No viewers found</p>
      </div>
    {:else}
      <table class="w-full">
        <thead class="bg-[var(--bg-light)] sticky top-0">
          <tr class="text-left text-[var(--text-secondary)] text-sm">
            <th class="px-4 py-3">Name</th>
            <th class="px-4 py-3">Reading</th>
            <th class="px-4 py-3">First seen</th>
            <th class="px-4 py-3">Last seen</th>
            <th class="px-4 py-3">Messages</th>
            <th class="px-4 py-3">Contribution</th>
            <th class="px-4 py-3">Tags</th>
            <th class="px-4 py-3">Notes</th>
          </tr>
        </thead>
        <tbody>
          {#each viewerStore.viewers as viewer (viewer.channel_id)}
            <tr
              class="border-b border-[var(--border-light)] hover:bg-[var(--bg-light)] cursor-pointer transition-colors {viewerStore.selectedViewer?.channel_id === viewer.channel_id ? 'bg-[var(--primary-start)]/10' : ''}"
              onclick={() => viewerStore.selectViewer(viewer)}
            >
              <td class="px-4 py-3">
                <div class="flex items-center gap-2">
                  <span class="text-[var(--text-primary)] font-medium">{viewer.display_name}</span>
                  {#if viewer.membership_level}
                    <span class="px-1.5 py-0.5 text-xs bg-green-100 text-green-700 rounded border border-green-200">
                      Member
                    </span>
                  {/if}
                </div>
              </td>
              <td class="px-4 py-3 text-[var(--text-secondary)]">
                {viewer.reading || '-'}
              </td>
              <td class="px-4 py-3 text-[var(--text-muted)] text-sm">
                {viewer.first_seen ? formatDate(viewer.first_seen) : '-'}
              </td>
              <td class="px-4 py-3 text-[var(--text-muted)] text-sm">
                {viewer.last_seen ? formatDate(viewer.last_seen) : '-'}
              </td>
              <td class="px-4 py-3 text-[var(--text-secondary)]">
                {viewer.message_count.toLocaleString()}
              </td>
              <td class="px-4 py-3 text-yellow-600">
                {formatContribution(viewer.total_contribution)}
              </td>
              <td class="px-4 py-3 text-[var(--text-secondary)] text-sm">
                {#if viewer.tags && viewer.tags.length > 0}
                  <div class="flex flex-wrap gap-1">
                    {#each viewer.tags.slice(0, 3) as tag}
                      <span class="px-1.5 py-0.5 text-xs bg-[var(--primary-start)]/10 text-[var(--primary-start)] rounded">{tag}</span>
                    {/each}
                    {#if viewer.tags.length > 3}
                      <span class="text-[var(--text-muted)]">+{viewer.tags.length - 3}</span>
                    {/if}
                  </div>
                {:else}
                  -
                {/if}
              </td>
              <td class="px-4 py-3 text-[var(--text-secondary)] text-sm max-w-[200px] truncate" title={viewer.notes || ''}>
                {viewer.notes || '-'}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>

  <!-- Pagination -->
  <div class="flex items-center justify-between px-4 py-3 bg-[var(--bg-light)] border-t border-[var(--border-light)]">
    <button
      onclick={() => viewerStore.prevPage()}
      disabled={viewerStore.currentPage === 0}
      class="px-4 py-2 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-white)] rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed border border-[var(--border-light)]"
    >
      Previous
    </button>
    <span class="text-[var(--text-muted)] text-sm">
      Page {viewerStore.currentPage + 1}
    </span>
    <button
      onclick={() => viewerStore.nextPage()}
      disabled={viewerStore.viewers.length < viewerStore.pageSize}
      class="px-4 py-2 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-white)] rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed border border-[var(--border-light)]"
    >
      Next
    </button>
  </div>
</div>
