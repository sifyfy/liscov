<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { ChatMessage } from '$lib/types';
  import { chatStore } from '$lib/stores';

  interface Props {
    viewer: {
      channelId: string;
      displayName: string;
      iconUrl?: string;
      message: ChatMessage;
    };
    broadcasterChannelId: string;
    onClose: () => void;
    onMessageClick?: (message: ChatMessage) => void;
  }

  let { viewer, broadcasterChannelId, onClose, onMessageClick }: Props = $props();

  let reading = $state('');
  let notes = $state('');
  let isSaving = $state(false);
  let saveMessage = $state('');
  let viewerProfileId = $state<number | null>(null);

  // Load existing custom info when viewer changes
  $effect(() => {
    // Explicitly reference reactive dependencies for effect tracking
    const bc = broadcasterChannelId;
    const vc = viewer.channelId;

    // Reset state before loading (important when viewer changes)
    reading = '';
    notes = '';
    saveMessage = '';
    viewerProfileId = null;

    // Load custom info for this viewer
    loadCustomInfo(bc, vc);
  });

  async function loadCustomInfo(bc: string, vc: string) {
    try {
      const profile = await invoke<{
        id: number;
      } | null>('viewer_get_profile', {
        broadcasterId: bc,
        channelId: vc
      });
      if (profile) {
        viewerProfileId = profile.id;
        // Direct DB lookup by viewer_profile_id (O(1) instead of scanning 1000 viewers)
        const customInfo = await invoke<{
          reading: string | null;
          notes: string | null;
        } | null>('viewer_get_custom_info', {
          viewerProfileId: profile.id
        });
        if (customInfo) {
          if (customInfo.reading !== null) {
            reading = customInfo.reading;
          }
          if (customInfo.notes !== null) {
            notes = customInfo.notes;
          }
        }
      }
    } catch (error) {
      console.error('Failed to load viewer info:', error);
    }
  }

  async function handleSave() {
    if (viewerProfileId === null) {
      saveMessage = '視聴者プロファイルが見つかりません';
      return;
    }

    isSaving = true;
    saveMessage = '';
    try {
      await invoke('viewer_upsert_custom_info', {
        viewerProfileId: viewerProfileId,
        reading: reading || null,
        notes: notes || null,
        customData: null
      });
      saveMessage = '保存しました';
      setTimeout(() => saveMessage = '', 3000);
    } catch (error) {
      console.error('Failed to save viewer info:', error);
      saveMessage = '保存に失敗しました';
    } finally {
      isSaving = false;
    }
  }

  // Get viewer's messages (O(1) lookup via channel index)
  let viewerMessages = $derived(
    chatStore.getMessagesForChannel(viewer.channelId)
  );

  function formatMessageType(msg: ChatMessage): { text: string; style: string } | null {
    if (msg.message_type === 'text') return null;
    if (msg.message_type === 'superchat') {
      return { text: `${msg.amount}`, style: 'bg-yellow-400/20 text-yellow-400' };
    }
    if (msg.message_type === 'supersticker') {
      return { text: `${msg.amount}`, style: 'bg-purple-400/20 text-purple-400' };
    }
    if (msg.message_type === 'membership') {
      return { text: 'メンバー', style: 'bg-[var(--success-subtle)] text-[var(--success)]' };
    }
    if (msg.message_type === 'membership_gift') {
      return { text: 'ギフト', style: 'bg-pink-400/20 text-pink-400' };
    }
    return null;
  }

  // Format timestamp to local timezone HH:MM:SS
  function formatTimestamp(timestamp: string): string {
    if (!timestamp) return '';
    try {
      const date = new Date(timestamp);
      if (isNaN(date.getTime())) {
        return timestamp;
      }
      return date.toLocaleTimeString('ja-JP', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
      });
    } catch {
      return timestamp;
    }
  }
</script>

<!-- Slide-in panel (original liscov dark theme style) -->
<div
  class="fixed right-0 top-0 h-full w-80 shadow-[-4px_0_12px_rgba(0,0,0,0.3)] z-50 flex flex-col animate-slide-in"
  style="background: var(--bg-surface-1);"
>
  <!-- Header (dark theme) -->
  <div
    class="flex items-center justify-between px-5 py-4 flex-shrink-0"
    style="background: var(--bg-surface-2); border-bottom: 1px solid var(--border-default);"
  >
    <h2 class="text-lg font-semibold text-[var(--text-primary)]">視聴者情報</h2>
    <button
      onclick={onClose}
      class="px-3 py-1 rounded text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors"
      style="background: var(--bg-surface-3); hover:background: #666;"
      title="閉じる"
    >
      ✕
    </button>
  </div>

  <!-- Content (dark theme) -->
  <div class="p-5 flex-1 flex flex-col overflow-hidden">
    <!-- Viewer info -->
    <div class="flex items-center gap-4 mb-4">
      {#if viewer.iconUrl}
        <img
          src={viewer.iconUrl}
          alt="視聴者アイコン"
          class="w-14 h-14 rounded-full"
        />
      {:else}
        <div class="w-14 h-14 rounded-full flex items-center justify-center text-[var(--text-muted)]" style="background: var(--bg-surface-3);"><svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" /></svg></div>
      {/if}
      <div>
        <p class="text-lg font-semibold text-[var(--text-primary)]">{viewer.displayName}</p>
        {#if reading}
          <p class="text-sm" style="color: var(--accent);">({reading})</p>
        {/if}
      </div>
    </div>

    <!-- Channel ID -->
    <p class="text-xs break-all mb-5" style="color: var(--text-muted);">
      Channel ID: {viewer.channelId}
    </p>

    <hr class="my-5" style="border-color: var(--border-default);" />

    <!-- Reading input -->
    <div class="mb-5">
      <label for="viewer-reading" class="block text-sm font-semibold text-[var(--text-primary)] mb-2">
        読み仮名（ふりがな）
      </label>
      <input
        id="viewer-reading"
        type="text"
        placeholder="例: やまだ たろう"
        bind:value={reading}
        class="w-full px-3 py-2 rounded-lg text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50"
        style="background: var(--bg-surface-3); border: 1px solid var(--border-default);"
      />
      <p class="text-xs mt-1" style="color: var(--text-muted);">
        視聴者名の横に括弧書きで表示されます
      </p>
    </div>

    <!-- Notes input -->
    <div class="mb-5">
      <label for="viewer-notes" class="block text-sm font-semibold text-[var(--text-primary)] mb-2">
        メモ
      </label>
      <textarea
        id="viewer-notes"
        placeholder="この視聴者についてのメモ..."
        bind:value={notes}
        rows="3"
        class="w-full px-3 py-2 rounded-lg text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50 resize-none"
        style="background: var(--bg-surface-3); border: 1px solid var(--border-default);"
      ></textarea>
    </div>

    <!-- Save button -->
    <div class="flex items-center gap-3 mb-5">
      <button
        onclick={handleSave}
        disabled={isSaving}
        class="flex-1 px-4 py-2 text-[var(--text-inverse)] rounded-lg transition-colors disabled:opacity-50"
        style="background: var(--accent);"
      >
        {isSaving ? '保存中...' : '保存'}
      </button>
      {#if saveMessage}
        <span class="text-sm" style="color: var(--success);">{saveMessage}</span>
      {/if}
    </div>

    <hr class="my-5" style="border-color: var(--border-default);" />

    <!-- Viewer's messages -->
    <div class="flex-1 flex flex-col min-h-0">
      <h3 class="text-sm font-semibold text-[var(--text-primary)] mb-3 flex-shrink-0">
        投稿されたコメント ({viewerMessages.length}件)
      </h3>
      <div class="flex-1 overflow-y-auto space-y-2">
        {#each [...viewerMessages].reverse() as message (message.id)}
          {@const isClicked = message.id === viewer.message.id}
          {@const badge = formatMessageType(message)}
          <button
            class="w-full text-left p-3 rounded-lg cursor-pointer transition-colors"
            style="background: var(--bg-surface-3); border: 1px solid {isClicked ? 'var(--accent)' : 'var(--border-default)'}; {isClicked ? 'box-shadow: 0 0 8px rgba(56, 189, 248, 0.4);' : ''}"
            onclick={() => onMessageClick?.(message)}
          >
            <p class="text-xs mb-1" style="color: var(--text-muted);">{formatTimestamp(message.timestamp)}</p>
            <p class="text-sm text-[var(--text-primary)] break-words leading-relaxed">{message.content}</p>
            {#if badge}
              <span class="inline-block mt-2 px-2 py-0.5 text-xs rounded {badge.style}">
                {badge.text}
              </span>
            {/if}
          </button>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  @keyframes slide-in {
    from {
      transform: translateX(100%);
    }
    to {
      transform: translateX(0);
    }
  }

  .animate-slide-in {
    animation: slide-in 0.25s ease-out;
  }
</style>
