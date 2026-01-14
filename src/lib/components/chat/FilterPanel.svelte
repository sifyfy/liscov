<script lang="ts">
  import { chatStore } from '$lib/stores';

  let showFilters = $state(false);
</script>

<div class="bg-[var(--bg-white)] border-b border-[var(--border-light)]">
  <!-- Toggle button -->
  <button
    onclick={() => (showFilters = !showFilters)}
    class="w-full px-4 py-2 flex items-center justify-between text-[var(--text-secondary)] hover:bg-[var(--bg-light)] transition-colors"
  >
    <span class="flex items-center gap-2">
      <svg
        class="w-4 h-4"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"
        />
      </svg>
      Filters
    </span>
    <svg
      class="w-4 h-4 transform transition-transform"
      class:rotate-180={showFilters}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  {#if showFilters}
    <div class="px-4 py-3 space-y-3 border-t border-[var(--border-light)]">
      <!-- Search -->
      <div>
        <input
          type="text"
          value={chatStore.filter.searchQuery}
          oninput={(e) => chatStore.setFilter({ searchQuery: e.currentTarget.value })}
          placeholder="Search messages..."
          class="w-full px-3 py-2 text-sm rounded-lg bg-[var(--bg-light)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-light)] focus:outline-none focus:ring-2 focus:ring-[var(--primary-start)]"
        />
      </div>

      <!-- Message type filters -->
      <div class="flex flex-wrap gap-2">
        <label class="flex items-center gap-2 px-3 py-1 bg-[var(--bg-light)] border border-[var(--border-light)] rounded-lg cursor-pointer hover:bg-gray-100">
          <input
            type="checkbox"
            checked={chatStore.filter.showText}
            onchange={(e) => chatStore.setFilter({ showText: e.currentTarget.checked })}
            class="w-4 h-4 rounded accent-[var(--primary-start)]"
          />
          <span class="text-sm text-[var(--text-primary)]">Text</span>
        </label>

        <label class="flex items-center gap-2 px-3 py-1 bg-yellow-50 border border-yellow-200 rounded-lg cursor-pointer hover:bg-yellow-100">
          <input
            type="checkbox"
            checked={chatStore.filter.showSuperchat}
            onchange={(e) => chatStore.setFilter({ showSuperchat: e.currentTarget.checked })}
            class="w-4 h-4 rounded accent-yellow-500"
          />
          <span class="text-sm text-yellow-800">Superchat</span>
        </label>

        <label class="flex items-center gap-2 px-3 py-1 bg-green-50 border border-green-200 rounded-lg cursor-pointer hover:bg-green-100">
          <input
            type="checkbox"
            checked={chatStore.filter.showMembership}
            onchange={(e) => chatStore.setFilter({ showMembership: e.currentTarget.checked })}
            class="w-4 h-4 rounded accent-green-500"
          />
          <span class="text-sm text-green-800">Membership</span>
        </label>
      </div>

      <!-- Chat mode -->
      <div class="flex items-center gap-4">
        <span class="text-sm text-[var(--text-muted)]">Chat mode:</span>
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            type="radio"
            name="chatMode"
            checked={chatStore.chatMode === 'top'}
            onchange={() => chatStore.setChatMode('top')}
            class="w-4 h-4 accent-[var(--primary-start)]"
          />
          <span class="text-sm text-[var(--text-primary)]">Top Chat</span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            type="radio"
            name="chatMode"
            checked={chatStore.chatMode === 'all'}
            onchange={() => chatStore.setChatMode('all')}
            class="w-4 h-4 accent-[var(--primary-start)]"
          />
          <span class="text-sm text-[var(--text-primary)]">All Chat</span>
        </label>
      </div>
    </div>
  {/if}
</div>
