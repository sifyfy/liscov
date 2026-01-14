<script lang="ts">
  import { analyticsStore } from '$lib/stores';
  import { onMount } from 'svelte';
  import type { SuperChatTier } from '$lib/types';

  let refreshInterval: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    // Initial load
    analyticsStore.loadAnalytics();

    // Auto-refresh every 30 seconds
    refreshInterval = setInterval(() => {
      analyticsStore.loadAnalytics();
    }, 30000);

    return () => {
      if (refreshInterval) {
        clearInterval(refreshInterval);
      }
    };
  });

  function formatNumber(num: number): string {
    return new Intl.NumberFormat('ja-JP').format(num);
  }

  // Tier color configuration (spec: 07_revenue.md)
  const tierConfig: Record<SuperChatTier, { label: string; bgColor: string; textColor: string }> = {
    Red: { label: 'Red', bgColor: 'bg-red-500', textColor: 'text-white' },
    Magenta: { label: 'Magenta', bgColor: 'bg-pink-500', textColor: 'text-white' },
    Orange: { label: 'Orange', bgColor: 'bg-orange-500', textColor: 'text-white' },
    Yellow: { label: 'Yellow', bgColor: 'bg-yellow-400', textColor: 'text-black' },
    Green: { label: 'Green', bgColor: 'bg-green-500', textColor: 'text-white' },
    Cyan: { label: 'Cyan', bgColor: 'bg-cyan-400', textColor: 'text-black' },
    Blue: { label: 'Blue', bgColor: 'bg-blue-500', textColor: 'text-white' },
  };

  // Get tier stats as array for rendering
  function getTierStats() {
    if (!analyticsStore.analytics) return [];
    const t = analyticsStore.analytics.super_chat_by_tier;
    return [
      { tier: 'Red' as SuperChatTier, count: t.tier_red },
      { tier: 'Magenta' as SuperChatTier, count: t.tier_magenta },
      { tier: 'Orange' as SuperChatTier, count: t.tier_orange },
      { tier: 'Yellow' as SuperChatTier, count: t.tier_yellow },
      { tier: 'Green' as SuperChatTier, count: t.tier_green },
      { tier: 'Cyan' as SuperChatTier, count: t.tier_cyan },
      { tier: 'Blue' as SuperChatTier, count: t.tier_blue },
    ];
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold text-[var(--text-primary)]">Revenue Analytics</h2>
    <button
      onclick={() => analyticsStore.loadAnalytics()}
      disabled={analyticsStore.isLoading}
      class="px-4 py-2 text-sm text-white rounded-lg transition-colors disabled:opacity-50"
      style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);"
    >
      {analyticsStore.isLoading ? 'Loading...' : 'Refresh'}
    </button>
  </div>

  {#if analyticsStore.error}
    <div class="p-4 bg-red-50 rounded-lg border border-red-200">
      <p class="text-red-600">{analyticsStore.error}</p>
    </div>
  {/if}

  {#if analyticsStore.analytics}
    <!-- Main stats grid -->
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <div class="p-4 bg-[var(--bg-white)] rounded-lg border border-[var(--border-light)] shadow-sm">
        <p class="text-sm text-[var(--text-muted)]">Total Paid Messages</p>
        <p class="text-2xl font-bold text-[var(--text-primary)]">
          {formatNumber(analyticsStore.totalPaidCount)}
        </p>
      </div>

      <div class="p-4 bg-[var(--bg-white)] rounded-lg border border-[var(--border-light)] shadow-sm">
        <p class="text-sm text-[var(--text-muted)]">Super Chats</p>
        <p class="text-2xl font-bold text-yellow-600">
          {formatNumber(analyticsStore.analytics.super_chat_count)}
        </p>
      </div>

      <div class="p-4 bg-[var(--bg-white)] rounded-lg border border-[var(--border-light)] shadow-sm">
        <p class="text-sm text-[var(--text-muted)]">Super Stickers</p>
        <p class="text-2xl font-bold text-pink-600">
          {formatNumber(analyticsStore.analytics.super_sticker_count)}
        </p>
      </div>

      <div class="p-4 bg-[var(--bg-white)] rounded-lg border border-[var(--border-light)] shadow-sm">
        <p class="text-sm text-[var(--text-muted)]">Memberships</p>
        <p class="text-2xl font-bold text-green-600">
          {formatNumber(analyticsStore.analytics.membership_gains)}
        </p>
      </div>
    </div>

    <!-- Tier Distribution -->
    <div class="p-4 bg-[var(--bg-white)] rounded-lg border border-[var(--border-light)] shadow-sm">
      <h3 class="text-lg font-medium text-[var(--text-primary)] mb-4">Super Chat Tier Distribution</h3>
      <div class="space-y-3">
        {#each getTierStats() as { tier, count }}
          {@const total = analyticsStore.totalTierCount || 1}
          {@const width = (count / total) * 100}
          {@const config = tierConfig[tier]}
          <div class="flex items-center gap-3">
            <span class="w-20 px-2 py-1 text-sm font-medium rounded {config.bgColor} {config.textColor} text-center">
              {config.label}
            </span>
            <div class="flex-1 h-6 bg-[var(--bg-light)] rounded overflow-hidden border border-[var(--border-light)]">
              <div class="{config.bgColor} h-full transition-all opacity-80" style="width: {width}%"></div>
            </div>
            <span class="w-12 text-sm font-medium text-[var(--text-primary)] text-right">{count}</span>
          </div>
        {/each}
      </div>
    </div>

    <!-- Top Contributors -->
    {#if analyticsStore.analytics.top_contributors.length > 0}
      <div class="p-4 bg-[var(--bg-white)] rounded-lg border border-[var(--border-light)] shadow-sm">
        <h3 class="text-lg font-medium text-[var(--text-primary)] mb-3">Top Contributors</h3>
        <div class="space-y-2">
          {#each analyticsStore.analytics.top_contributors as contributor, index}
            {@const config = tierConfig[contributor.highest_tier]}
            <div class="flex items-center gap-3 py-2 {index !== analyticsStore.analytics.top_contributors.length - 1 ? 'border-b border-[var(--border-light)]' : ''}">
              <span class="w-6 h-6 flex items-center justify-center text-sm font-bold rounded-full {index === 0 ? 'bg-yellow-500 text-black' : index === 1 ? 'bg-gray-400 text-black' : index === 2 ? 'bg-orange-600 text-white' : 'bg-[var(--bg-light)] text-[var(--text-primary)]'}">
                {index + 1}
              </span>
              <span class="flex-1 text-[var(--text-primary)] font-medium">{contributor.display_name}</span>
              <span class="px-2 py-0.5 text-xs font-medium rounded {config.bgColor} {config.textColor}">
                {config.label}
              </span>
              <span class="text-[var(--text-muted)] text-sm">({contributor.contribution_count}x)</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Last update -->
    {#if analyticsStore.lastUpdate}
      <p class="text-sm text-[var(--text-muted)] text-right">
        Last updated: {analyticsStore.lastUpdate.toLocaleTimeString('ja-JP')}
      </p>
    {/if}
  {:else if !analyticsStore.isLoading}
    <div class="flex items-center justify-center h-32">
      <p class="text-[var(--text-muted)]">No analytics data available</p>
    </div>
  {/if}
</div>
