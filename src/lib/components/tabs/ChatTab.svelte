<script lang="ts">
  import { ChatDisplay, ConnectionList, FilterPanel, InputSection } from '$lib/components/chat';
  import { chatStore } from '$lib/stores';

  // 接続リストパネルの高さ管理
  const MAX_HEIGHT_RATIO = 0.4; // ビューポートの40%まで
  const DEFAULT_HEIGHT = 120;

  let panelHeight = $state(DEFAULT_HEIGHT);
  let isDragging = $state(false);
  let containerEl = $state<HTMLElement | undefined>();

  let hasConnections = $derived(chatStore.connections.size > 0);

  // 接続アイテム1件分の実測高さを下限として使う
  function getMinHeight(): number {
    // 内側の .connection-list（実際にpaddingを持つ要素）を取得
    const innerList = containerEl?.querySelector('.connection-list');
    const firstItem = innerList?.querySelector('.connection-item');
    if (!innerList || !firstItem) return 50; // フォールバック
    const style = getComputedStyle(innerList);
    const paddingTop = parseFloat(style.paddingTop) || 0;
    const paddingBottom = parseFloat(style.paddingBottom) || 0;
    return firstItem.getBoundingClientRect().height + paddingTop + paddingBottom;
  }

  function handlePointerDown(e: PointerEvent) {
    isDragging = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging || !containerEl) return;
    const containerRect = containerEl.getBoundingClientRect();
    const maxHeight = containerRect.height * MAX_HEIGHT_RATIO;
    const listTop = containerEl.querySelector('[data-connection-list]')?.getBoundingClientRect().top ?? containerRect.top;
    const newHeight = e.clientY - listTop;
    const minHeight = getMinHeight();
    panelHeight = Math.max(minHeight, Math.min(newHeight, maxHeight));
  }

  function handlePointerUp() {
    isDragging = false;
  }
</script>

<div
  bind:this={containerEl}
  class="flex flex-col h-full overflow-hidden"
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerUp}
>
  <!-- 接続設定（固定、スクロール不可） -->
  <div class="flex-shrink-0 bg-[var(--bg-surface-1)] border-b" style="border-color: var(--border-subtle);">
    <InputSection />
  </div>

  <!-- 接続中リスト（接続がある場合のみ表示） -->
  {#if hasConnections}
    <!-- ヘッダー（固定、スクロール不可） -->
    <div class="flex-shrink-0 flex justify-between items-center px-3 py-1 bg-[var(--bg-surface-1)]">
      <span class="text-xs text-[var(--text-secondary)]">接続中: {chatStore.connections.size}</span>
      {#if chatStore.connections.size > 1}
        <button
          class="text-xs px-2 py-0.5 rounded bg-[var(--bg-surface-2)] text-[var(--text-secondary)] border border-[var(--border-default)] hover:bg-[var(--bg-surface-3)] cursor-pointer"
          onclick={() => chatStore.disconnectAll()}
        >全切断</button>
      {/if}
    </div>

    <!-- 接続アイテム（スクロール可能） -->
    <div
      data-connection-list
      class="flex-shrink-0 overflow-y-auto"
      style="max-height: {panelHeight}px;"
    >
      <ConnectionList />
    </div>

    <!-- リサイズハンドル -->
    <div
      class="flex-shrink-0 flex items-center justify-center h-1.5 cursor-row-resize select-none transition-colors
             {isDragging ? 'bg-[var(--accent)]' : 'bg-[var(--border-default)] hover:bg-[var(--accent-subtle)]'}"
      onpointerdown={handlePointerDown}
      role="separator"
      aria-orientation="horizontal"
      aria-label="接続リストのサイズ変更"
    >
      <div class="w-8 h-0.5 rounded-full {isDragging ? 'bg-white/60' : 'bg-[var(--text-muted)]'}"></div>
    </div>
  {/if}

  <!-- 下部パネル: フィルター + チャット表示 -->
  <FilterPanel />
  <div class="flex-1 overflow-hidden">
    <ChatDisplay />
  </div>
</div>
