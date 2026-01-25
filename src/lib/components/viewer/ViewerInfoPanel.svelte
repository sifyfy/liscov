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
      // Use viewer_get_profile to get viewer profile including id
      const profile = await invoke<{
        id: number;
        reading?: string | null;
        notes?: string | null;
      } | null>('viewer_get_profile', {
        broadcasterId: bc,
        channelId: vc
      });
      if (profile) {
        viewerProfileId = profile.id;
        // viewer_get_profile doesn't include reading/notes, need to get from viewer_get_list
        // For now, try to load from the full list
        const viewers = await invoke<Array<{
          id: number;
          reading: string | null;
          notes: string | null;
        }>>('viewer_get_list', {
          broadcasterId: bc,
          searchQuery: null,
          limit: 1000,
          offset: 0
        });
        const viewerInfo = viewers.find(v => v.id === profile.id);
        if (viewerInfo) {
          if (viewerInfo.reading !== null) {
            reading = viewerInfo.reading;
          }
          if (viewerInfo.notes !== null) {
            notes = viewerInfo.notes;
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

  // Get viewer's messages
  let viewerMessages = $derived(
    chatStore.messages.filter(m => m.channel_id === viewer.channelId)
  );

  function formatMessageType(msg: ChatMessage): { text: string; style: string } | null {
    if (msg.message_type === 'text') return null;
    if (msg.message_type === 'superchat') {
      return { text: `${msg.amount}`, style: 'bg-yellow-100 text-yellow-800' };
    }
    if (msg.message_type === 'supersticker') {
      return { text: `${msg.amount}`, style: 'bg-purple-100 text-purple-800' };
    }
    if (msg.message_type === 'membership') {
      return { text: 'メンバー', style: 'bg-green-100 text-green-800' };
    }
    if (msg.message_type === 'membership_gift') {
      return { text: 'ギフト', style: 'bg-pink-100 text-pink-800' };
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
  class="fixed right-0 top-0 h-full w-80 shadow-[-4px_0_12px_rgba(0,0,0,0.3)] z-50 overflow-y-auto animate-slide-in"
  style="background: #2d2d3d;"
>
  <!-- Header (dark theme) -->
  <div
    class="flex items-center justify-between px-5 py-4"
    style="background: #363648; border-bottom: 1px solid #555;"
  >
    <h2 class="text-lg font-semibold text-white">視聴者情報</h2>
    <button
      onclick={onClose}
      class="px-3 py-1 rounded text-white transition-colors"
      style="background: #555; hover:background: #666;"
      title="閉じる"
    >
      ✕
    </button>
  </div>

  <!-- Content (dark theme) -->
  <div class="p-5">
    <!-- Viewer info -->
    <div class="flex items-center gap-4 mb-4">
      {#if viewer.iconUrl}
        <img
          src={viewer.iconUrl}
          alt="視聴者アイコン"
          class="w-14 h-14 rounded-full"
        />
      {:else}
        <div class="w-14 h-14 rounded-full flex items-center justify-center text-2xl" style="background: #3d3d4d;">
          👤
        </div>
      {/if}
      <div>
        <p class="text-lg font-semibold text-white">{viewer.displayName}</p>
        {#if reading}
          <p class="text-sm" style="color: #a78bfa;">({reading})</p>
        {/if}
      </div>
    </div>

    <!-- Channel ID -->
    <p class="text-xs break-all mb-5" style="color: #9ca3af;">
      Channel ID: {viewer.channelId}
    </p>

    <hr class="my-5" style="border-color: #555;" />

    <!-- Reading input -->
    <div class="mb-5">
      <label for="viewer-reading" class="block text-sm font-semibold text-white mb-2">
        読み仮名（ふりがな）
      </label>
      <input
        id="viewer-reading"
        type="text"
        placeholder="例: やまだ たろう"
        bind:value={reading}
        class="w-full px-3 py-2 rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-purple-400/50"
        style="background: #3d3d4d; border: 1px solid #555;"
      />
      <p class="text-xs mt-1" style="color: #9ca3af;">
        視聴者名の横に括弧書きで表示されます
      </p>
    </div>

    <!-- Notes input -->
    <div class="mb-5">
      <label for="viewer-notes" class="block text-sm font-semibold text-white mb-2">
        メモ
      </label>
      <textarea
        id="viewer-notes"
        placeholder="この視聴者についてのメモ..."
        bind:value={notes}
        rows="3"
        class="w-full px-3 py-2 rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-purple-400/50 resize-none"
        style="background: #3d3d4d; border: 1px solid #555;"
      ></textarea>
    </div>

    <!-- Save button -->
    <div class="flex items-center gap-3 mb-5">
      <button
        onclick={handleSave}
        disabled={isSaving}
        class="flex-1 px-4 py-2 text-white rounded-lg transition-colors disabled:opacity-50"
        style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);"
      >
        {isSaving ? '保存中...' : '保存'}
      </button>
      {#if saveMessage}
        <span class="text-sm" style="color: #4ade80;">{saveMessage}</span>
      {/if}
    </div>

    <hr class="my-5" style="border-color: #555;" />

    <!-- Viewer's messages -->
    <div>
      <h3 class="text-sm font-semibold text-white mb-3">
        投稿されたコメント ({viewerMessages.length}件)
      </h3>
      <div class="max-h-72 overflow-y-auto space-y-2">
        {#each [...viewerMessages].reverse() as message (message.id)}
          {@const isClicked = message.id === viewer.message.id}
          {@const badge = formatMessageType(message)}
          <button
            class="w-full text-left p-3 rounded-lg cursor-pointer transition-colors"
            style="background: #3d3d4d; border: 1px solid {isClicked ? '#a78bfa' : '#555'}; {isClicked ? 'box-shadow: 0 0 8px rgba(167, 139, 250, 0.4);' : ''}"
            onclick={() => onMessageClick?.(message)}
          >
            <p class="text-xs mb-1" style="color: #9ca3af;">{formatTimestamp(message.timestamp)}</p>
            <p class="text-sm text-white break-words leading-relaxed">{message.content}</p>
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
