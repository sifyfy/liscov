<script lang="ts">
  import { viewerStore } from '$lib/stores';
  import type { BroadcasterChannel } from '$lib/types';

  interface Props {
    selected: string | null;
    onSelect: (broadcasterId: string | null) => void;
  }

  let { selected, onSelect }: Props = $props();

  $effect(() => {
    viewerStore.loadBroadcasters();
  });

  function handleChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    const value = target.value || null;
    onSelect(value);
  }

  function getDisplayName(broadcaster: BroadcasterChannel): string {
    if (broadcaster.channel_name) {
      return broadcaster.channel_name;
    }
    if (broadcaster.handle) {
      return broadcaster.handle;
    }
    return broadcaster.channel_id;
  }
</script>

<div class="flex items-center gap-3">
  <label for="broadcaster-select" class="text-sm text-[var(--text-secondary)] whitespace-nowrap">
    Broadcaster:
  </label>
  <select
    id="broadcaster-select"
    value={selected || ''}
    onchange={handleChange}
    class="flex-1 px-3 py-2 rounded-lg bg-[var(--bg-white)] text-[var(--text-primary)] border border-[var(--border-light)] focus:outline-none focus:ring-2 focus:ring-[var(--primary-start)]/50"
  >
    <option value="">Select a broadcaster...</option>
    {#each viewerStore.broadcasters as broadcaster (broadcaster.channel_id)}
      <option value={broadcaster.channel_id}>
        {getDisplayName(broadcaster)} ({broadcaster.viewer_count} viewers)
      </option>
    {/each}
  </select>
</div>
