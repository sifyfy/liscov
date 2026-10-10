<script lang="ts">
  /**
   * 視聴者のタブ (docs/specs/06_viewer.md「視聴者のタブ (配信の後・準備)」)
   *
   * 配信の後や準備のときに、視聴者の読み仮名・メモ・タグを続けて整理する画面。試作 src/routes/design-proto/viewers の C。
   * - 配信者は主の配信の配信者 (props)。接続していないか、その配信者の記録がまだなければ記録のある最初の配信者
   * - 探す欄は打ち終えて 300ms で絞る。「読み仮名なしだけ」はすぐ絞る。どちらも最初のページに戻る (viewerStore.setFilter)
   * - 編集で保存した読み仮名とメモは、チャットの画面の同じ視聴者の行にもすぐ出す (chatStore.setViewerInfo)
   */
  import { onDestroy, onMount, untrack } from 'svelte';
  import { chatStore, viewerStore } from '$lib/stores';
  import { normalizeError } from '$lib/tauri/errors';
  import type { BroadcasterChannel, ViewerWithCustomInfo } from '$lib/types';
  import { formatCount } from '$lib/utils/format';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ViewerList from './ViewerList.svelte';
  import DeleteConfirmDialog from './DeleteConfirmDialog.svelte';

  interface Props {
    /** 主の配信の配信者チャンネル ID (接続していなければ undefined) */
    broadcasterId?: string;
  }

  let { broadcasterId: primaryBroadcasterId }: Props = $props();

  const SEARCH_DEBOUNCE_MS = 300;

  let selectedId = $state<string | null>(null);
  let broadcastersLoaded = $state(false);
  // 絞り込みはストアにあるので、タブを開き直しても前の条件のまま
  let searchText = $state(untrack(() => viewerStore.searchQuery));
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let confirmingBroadcasterDelete = $state(false);
  let deletingBroadcaster = $state(false);
  let broadcasterDeleteError = $state<string | null>(null);

  const selectedBroadcaster = $derived(viewerStore.broadcasters.find((b) => b.channel_id === selectedId) ?? null);
  const filtered = $derived(viewerStore.searchQuery.trim() !== '' || viewerStore.withoutReading);
  const pageStart = $derived(viewerStore.currentPage * viewerStore.pageSize);
  const hasNextPage = $derived(viewerStore.viewers.length === viewerStore.pageSize);

  const nameOf = (b: BroadcasterChannel) => b.channel_name || b.handle || b.channel_id;

  function select(id: string | null) {
    selectedId = id;
    if (id) void viewerStore.loadViewers(id, 0);
  }

  /** 配信者の一覧を読み直し、選んでいた配信者 (いなければ preferred → 最初の配信者) を選ぶ */
  async function loadBroadcasters(preferred: string | null) {
    await viewerStore.loadBroadcasters();
    broadcastersLoaded = true;
    const ids = viewerStore.broadcasters.map((b) => b.channel_id);
    select([selectedId, preferred].find((id) => id && ids.includes(id)) ?? ids[0] ?? null);
  }

  onMount(() => loadBroadcasters(primaryBroadcasterId ?? null));
  onDestroy(() => clearTimeout(searchTimer));

  function handleSearchInput() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      if (selectedId) void viewerStore.setFilter(selectedId, { search: searchText });
    }, SEARCH_DEBOUNCE_MS);
  }

  function handleWithoutReadingChange(e: Event) {
    if (selectedId) void viewerStore.setFilter(selectedId, { withoutReading: (e.currentTarget as HTMLInputElement).checked });
  }

  async function handleSave(viewer: ViewerWithCustomInfo, values: { reading: string; notes: string; tags: string[] }) {
    await viewerStore.updateViewerInfo(viewer.id, values.reading || null, values.notes || null, viewer.custom_data, values.tags);
    chatStore.setViewerInfo(viewer.broadcaster_channel_id, viewer.channel_id, { reading: values.reading, notes: values.notes });
  }

  async function handleDelete(viewer: ViewerWithCustomInfo) {
    await viewerStore.deleteViewer(viewer.id);
  }

  async function handleDeleteBroadcaster() {
    if (!selectedId) return;
    deletingBroadcaster = true;
    broadcasterDeleteError = null;
    try {
      // ストアが配信者の一覧を読み直す。消したら記録のある最初の配信者を選ぶ
      await viewerStore.deleteBroadcaster(selectedId);
      confirmingBroadcasterDelete = false;
      select(viewerStore.broadcasters[0]?.channel_id ?? null);
    } catch (e) {
      broadcasterDeleteError = `消すのに失敗しました: ${normalizeError(e).message}`;
    } finally {
      deletingBroadcaster = false;
    }
  }
</script>

<div class="flex min-h-0 flex-1 flex-col bg-surface text-fg-1">
  {#if !broadcastersLoaded}
    <p class="p-4 text-label text-fg-2">読み込み中…</p>
  {:else if viewerStore.broadcasters.length === 0}
    <p class="p-4 text-label text-fg-2">まだ記録がありません。配信に接続してコメントが届くと記録されます</p>
  {:else}
    <!-- 配信者の選択・消す、探す・読み仮名なしだけ・読み込み直す -->
    <div class="flex shrink-0 flex-col gap-2 border-b border-outline-variant px-3 py-2">
      <div class="flex flex-wrap items-center gap-2">
        <label class="flex min-w-0 flex-1 items-center gap-2 text-label text-fg-2">
          <span class="shrink-0">配信者</span>
          <select
            value={selectedId ?? ''}
            onchange={(e) => select(e.currentTarget.value || null)}
            class="min-w-0 flex-1 rounded-control border border-outline bg-surface px-2 py-1 text-label text-fg-1"
          >
            {#each viewerStore.broadcasters as b (b.channel_id)}
              <option value={b.channel_id}>{nameOf(b)} ({formatCount(b.viewer_count)} 人)</option>
            {/each}
          </select>
        </label>
        <button
          type="button"
          onclick={() => {
            broadcasterDeleteError = null;
            confirmingBroadcasterDelete = true;
          }}
          disabled={!selectedBroadcaster}
          class="shrink-0 rounded-control px-2 py-1 text-label whitespace-nowrap text-error hover:bg-fg-1/8 disabled:opacity-50"
        >
          この配信者の記録を消す…
        </button>
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <span class="relative min-w-40 flex-1">
          <span class="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-fg-3"><Icon name="search" size={14} /></span>
          <input
            type="search"
            aria-label="視聴者を探す"
            placeholder="名前・読み仮名・メモで探す"
            bind:value={searchText}
            oninput={handleSearchInput}
            class="w-full rounded-control border border-outline bg-surface py-1 pr-2 pl-7 text-label text-fg-1 placeholder:text-fg-3"
          />
        </span>
        <label class="flex shrink-0 items-center gap-1 text-label text-fg-2">
          <input type="checkbox" checked={viewerStore.withoutReading} onchange={handleWithoutReadingChange} class="accent-accent" />
          読み仮名なしだけ
        </label>
        <button
          type="button"
          aria-label="読み込み直す"
          title="読み込み直す"
          onclick={() => loadBroadcasters(null)}
          class="flex size-8 shrink-0 items-center justify-center rounded-control text-fg-2 hover:bg-fg-1/8"
        >
          <Icon name="refresh" size={16} />
        </button>
      </div>
    </div>

    {#if viewerStore.error}
      <p role="alert" class="shrink-0 px-3 py-1 text-caption text-error">{viewerStore.error}</p>
    {/if}

    {#if viewerStore.viewers.length === 0}
      <p class="p-4 text-label text-fg-2">
        {#if viewerStore.isLoading}
          読み込み中…
        {:else if filtered}
          見つかりません
        {:else}
          まだ視聴者の記録がありません
        {/if}
      </p>
    {:else}
      <ViewerList viewers={viewerStore.viewers} onSave={handleSave} onDelete={handleDelete} />
    {/if}

    {#if viewerStore.viewers.length > 0 || viewerStore.currentPage > 0}
      <div class="flex h-9 shrink-0 items-center justify-between border-t border-outline-variant px-3 text-label text-fg-2">
        <button
          type="button"
          onclick={() => viewerStore.prevPage()}
          disabled={viewerStore.currentPage === 0}
          class="rounded-control px-2 py-1 hover:bg-fg-1/8 disabled:opacity-50"
        >
          前の {viewerStore.pageSize} 人
        </button>
        {#if viewerStore.viewers.length > 0}
          <span class="tabular-nums">{pageStart + 1}〜{pageStart + viewerStore.viewers.length} 人目</span>
        {/if}
        <button
          type="button"
          onclick={() => viewerStore.nextPage()}
          disabled={!hasNextPage}
          class="rounded-control px-2 py-1 hover:bg-fg-1/8 disabled:opacity-50"
        >
          次の {viewerStore.pageSize} 人
        </button>
      </div>
    {/if}
  {/if}
</div>

{#if confirmingBroadcasterDelete && selectedBroadcaster}
  <DeleteConfirmDialog
    title="この配信者の記録を消す"
    message="取り消せません。{nameOf(selectedBroadcaster)} のもとの視聴者 {formatCount(selectedBroadcaster.viewer_count)} 人の記録 (読み仮名・メモ・タグ・コメント数など) をすべて消します"
    isDeleting={deletingBroadcaster}
    error={broadcasterDeleteError}
    onConfirm={handleDeleteBroadcaster}
    onCancel={() => (confirmingBroadcasterDelete = false)}
  />
{/if}
