<script lang="ts">
  import { chatStore } from '$lib/stores';

  let showFilterPanel = $state(false);
  let showClearConfirm = $state(false);

  function scrollToLatest() {
    chatStore.setAutoScroll(true);
    chatStore.scrollToLatest();
  }

  function handleClearMessages() {
    showClearConfirm = true;
  }

  function confirmClear() {
    chatStore.clearMessages();
    showClearConfirm = false;
  }

  function cancelClear() {
    showClearConfirm = false;
  }

  // Calculate filtered message count
  let filteredCount = $derived(chatStore.filteredMessages.length);
  let displayLimitLabel = $derived(chatStore.displayLimit ? `${chatStore.displayLimit}件` : '無制限');
</script>

<!-- Original liscov style: Control bar -->
<div class="bg-[var(--bg-surface-1)] border-b" style="border-color: var(--border-subtle);">
  <!-- Main control bar (1 row) -->
  <div class="flex items-center gap-2 px-3 py-2">
    <!-- Filter toggle button -->
    <button
      onclick={() => (showFilterPanel = !showFilterPanel)}
      class="flex items-center gap-1.5 px-3 py-1 text-sm rounded border border-[var(--border-default)] bg-[var(--accent-subtle)] text-[var(--accent)] hover:bg-[var(--bg-surface-3)] transition-colors"
    >
      フィルター
    </button>

    <!-- Scroll to latest button -->
    <button
      onclick={scrollToLatest}
      class="flex items-center gap-1.5 px-3 py-1 text-sm rounded border border-[var(--border-default)] bg-[var(--success-subtle)] text-[var(--success)] hover:bg-[var(--bg-surface-3)] transition-colors"
    >
      最新に戻る
    </button>

    <!-- Auto scroll checkbox -->
    <label class="flex items-center gap-1.5 cursor-pointer">
      <input
        type="checkbox"
        checked={chatStore.autoScroll}
        onchange={(e) => chatStore.setAutoScroll(e.currentTarget.checked)}
        class="w-4 h-4 rounded accent-[var(--accent)]"
      />
      <span class="text-sm text-[var(--text-primary)]">自動スクロール</span>
    </label>

    <!-- Timestamp checkbox -->
    <label class="flex items-center gap-1.5 cursor-pointer">
      <input
        type="checkbox"
        checked={chatStore.showTimestamps}
        onchange={(e) => chatStore.setShowTimestamps(e.currentTarget.checked)}
        class="w-4 h-4 rounded accent-[var(--accent)]"
      />
      <span class="text-sm text-[var(--text-primary)]">タイムスタンプ</span>
    </label>

    <!-- Font size controls -->
    <div class="flex items-center gap-1 ml-auto">
      <button
        onclick={() => chatStore.decreaseFontSize()}
        class="w-7 h-7 flex items-center justify-center text-sm rounded border border-[var(--border-default)] bg-[var(--bg-surface-2)] text-[var(--text-secondary)] hover:bg-[var(--bg-surface-3)] transition-colors"
        title="文字を小さく"
      >
        A-
      </button>
      <span class="text-xs text-[var(--text-muted)] w-8 text-center">{chatStore.messageFontSize}px</span>
      <button
        onclick={() => chatStore.increaseFontSize()}
        class="w-7 h-7 flex items-center justify-center text-sm rounded border border-[var(--border-default)] bg-[var(--bg-surface-2)] text-[var(--text-secondary)] hover:bg-[var(--bg-surface-3)] transition-colors"
        title="文字を大きく"
      >
        A+
      </button>
    </div>

    <!-- Display limit selector -->
    <div class="flex items-center gap-1.5">
      <span class="text-sm text-[var(--text-muted)]">表示:</span>
      <select
        class="px-2 py-1 text-sm rounded border border-[var(--border-default)] bg-[var(--bg-surface-2)] text-[var(--text-primary)]"
        value={chatStore.displayLimit || 'unlimited'}
        onchange={(e) => chatStore.setDisplayLimit(e.currentTarget.value === 'unlimited' ? null : parseInt(e.currentTarget.value))}
      >
        <option value="unlimited">無制限</option>
        <option value="50">50件</option>
        <option value="100">100件</option>
        <option value="200">200件</option>
        <option value="500">500件</option>
      </select>
    </div>

    <!-- Clear messages button -->
    <button
      onclick={handleClearMessages}
      disabled={chatStore.messages.length === 0}
      class="flex items-center gap-1.5 px-3 py-1 text-sm rounded border border-[var(--border-default)] bg-[var(--error-subtle)] text-[var(--error)] hover:bg-[var(--bg-surface-3)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    >
      クリア
    </button>
  </div>

  <!-- Status bar (1 row) -->
  <div class="flex items-center gap-6 px-3 py-1.5 bg-[var(--bg-surface-2)] border-t text-xs text-[var(--text-muted)]" style="border-color: var(--border-subtle);">
    <span>フィルタ後: {filteredCount}件 / 表示枠: {displayLimitLabel}</span>
    <span class="ml-auto">全{chatStore.messages.length}件</span>
  </div>

  <!-- Expandable filter panel -->
  {#if showFilterPanel}
    <div class="px-3 py-3 space-y-3 border-t border-[var(--border-default)] bg-[var(--bg-surface-2)]">
      <!-- Search -->
      <div>
        <input
          type="text"
          value={chatStore.filter.searchQuery}
          oninput={(e) => chatStore.setFilter({ searchQuery: e.currentTarget.value })}
          placeholder="メッセージを検索..."
          class="w-full px-3 py-2 text-sm rounded bg-[var(--bg-surface-3)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50"
        />
      </div>

      <!-- Message type filters -->
      <div class="flex flex-wrap gap-2">
        <label class="flex items-center gap-2 px-3 py-1 bg-[var(--bg-surface-3)] border border-[var(--border-default)] rounded cursor-pointer hover:bg-[var(--bg-surface-3)]">
          <input
            type="checkbox"
            checked={chatStore.filter.showText}
            onchange={(e) => chatStore.setFilter({ showText: e.currentTarget.checked })}
            class="w-4 h-4 rounded accent-[var(--accent)]"
          />
          <span class="text-sm text-[var(--text-primary)]">💬 通常</span>
        </label>

        <label class="flex items-center gap-2 px-3 py-1 bg-[var(--bg-surface-3)] border border-[var(--border-default)] rounded cursor-pointer hover:bg-[var(--bg-surface-3)]">
          <input
            type="checkbox"
            checked={chatStore.filter.showSuperchat}
            onchange={(e) => chatStore.setFilter({ showSuperchat: e.currentTarget.checked })}
            class="w-4 h-4 rounded accent-[var(--accent)]"
          />
          <span class="text-sm text-[var(--text-primary)]">💰 SuperChat</span>
        </label>

        <label class="flex items-center gap-2 px-3 py-1 bg-[var(--bg-surface-3)] border border-[var(--border-default)] rounded cursor-pointer hover:bg-[var(--bg-surface-3)]">
          <input
            type="checkbox"
            checked={chatStore.filter.showMembership}
            onchange={(e) => chatStore.setFilter({ showMembership: e.currentTarget.checked })}
            class="w-4 h-4 rounded accent-[var(--accent)]"
          />
          <span class="text-sm text-[var(--text-primary)]">⭐ メンバー</span>
        </label>
      </div>
    </div>
  {/if}
</div>

<!-- Clear confirmation dialog -->
{#if showClearConfirm}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-[var(--bg-elevated)] rounded-lg shadow-xl p-6 max-w-sm mx-4">
      <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-2">メッセージをクリア</h3>
      <p class="text-sm text-[var(--text-secondary)] mb-4">
        {chatStore.messages.length}件のメッセージをすべて削除しますか？<br/>
        この操作は取り消せません。
      </p>
      <div class="flex justify-end gap-2">
        <button
          onclick={cancelClear}
          class="px-4 py-2 text-sm rounded border border-[var(--border-default)] text-[var(--text-secondary)] hover:bg-[var(--bg-surface-3)] transition-colors"
        >
          キャンセル
        </button>
        <button
          onclick={confirmClear}
          class="px-4 py-2 text-sm rounded bg-[var(--error)] text-[var(--text-inverse)] hover:opacity-90 transition-colors"
        >
          クリア
        </button>
      </div>
    </div>
  </div>
{/if}
