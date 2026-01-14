<script lang="ts">
  import { chatStore } from '$lib/stores';
  import ChatMessageComponent from './ChatMessage.svelte';
  import { ViewerInfoPanel } from '$lib/components/viewer';
  import type { ChatMessage } from '$lib/types';

  let chatContainer: HTMLDivElement;
  let autoScroll = $state(true);

  // Selected viewer for ViewerInfoPanel
  let selectedViewer = $state<{
    channelId: string;
    displayName: string;
    iconUrl?: string;
    message: ChatMessage;
  } | null>(null);

  // Auto-scroll when new messages arrive
  $effect(() => {
    const messages = chatStore.filteredMessages;
    if (autoScroll && chatContainer && messages.length > 0) {
      requestAnimationFrame(() => {
        chatContainer.scrollTop = chatContainer.scrollHeight;
      });
    }
  });

  function handleScroll() {
    if (!chatContainer) return;
    const { scrollTop, scrollHeight, clientHeight } = chatContainer;
    // Enable auto-scroll if user scrolls near the bottom
    autoScroll = scrollHeight - scrollTop - clientHeight < 100;
  }

  function handleMessageClick(message: ChatMessage) {
    selectedViewer = {
      channelId: message.channel_id,
      displayName: message.author,
      iconUrl: message.author_icon_url || undefined,
      message: message
    };
  }

  function closeViewerPanel() {
    selectedViewer = null;
  }

  function handleViewerMessageClick(message: ChatMessage) {
    // Update selected message within the same viewer
    if (selectedViewer) {
      selectedViewer = {
        ...selectedViewer,
        message: message
      };
    }
  }
</script>

<div class="flex flex-col h-full bg-[var(--bg-light)]">
  <!-- Header -->
  <div class="flex items-center justify-between px-4 py-2 bg-[var(--bg-white)] border-b border-[var(--border-light)]">
    <div class="flex items-center gap-2">
      <span class="font-semibold text-[var(--text-primary)]">Chat</span>
      <span class="text-[var(--text-muted)] text-sm">
        ({chatStore.filteredMessages.length} messages)
      </span>
    </div>
    <div class="flex items-center gap-3">
      <!-- Font size controls -->
      <div class="flex items-center gap-1 text-xs text-[var(--text-muted)]">
        <button
          onclick={() => chatStore.decreaseFontSize()}
          class="w-6 h-6 flex items-center justify-center hover:bg-[var(--bg-light)] rounded transition-colors"
          title="文字サイズを小さく"
        >
          A-
        </button>
        <span class="w-8 text-center">{chatStore.messageFontSize}px</span>
        <button
          onclick={() => chatStore.increaseFontSize()}
          class="w-6 h-6 flex items-center justify-center hover:bg-[var(--bg-light)] rounded transition-colors"
          title="文字サイズを大きく"
        >
          A+
        </button>
      </div>
      <!-- Timestamp toggle -->
      <label class="flex items-center gap-1 text-xs text-[var(--text-muted)] cursor-pointer">
        <input
          type="checkbox"
          checked={chatStore.showTimestamps}
          onchange={(e) => chatStore.setShowTimestamps((e.target as HTMLInputElement).checked)}
          class="w-3 h-3"
        />
        時刻
      </label>
      <!-- Clear button -->
      <button
        onclick={() => chatStore.clearMessages()}
        class="px-3 py-1 text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-light)] rounded transition-colors border border-transparent hover:border-[var(--border-light)]"
      >
        Clear
      </button>
    </div>
  </div>

  <!-- Messages -->
  <div
    bind:this={chatContainer}
    onscroll={handleScroll}
    class="flex-1 overflow-y-auto p-3 space-y-2"
  >
    {#if chatStore.filteredMessages.length === 0}
      <div class="flex items-center justify-center h-full">
        <p class="text-[var(--text-muted)] text-center">
          {#if chatStore.isConnected}
            Waiting for messages...
          {:else}
            Connect to a stream to see chat messages
          {/if}
        </p>
      </div>
    {:else}
      {#each chatStore.filteredMessages as message (message.id)}
        <ChatMessageComponent {message} onClick={() => handleMessageClick(message)} />
      {/each}
    {/if}
  </div>

  <!-- Viewer Info Panel -->
  {#if selectedViewer && chatStore.broadcasterChannelId}
    <ViewerInfoPanel
      viewer={selectedViewer}
      broadcasterChannelId={chatStore.broadcasterChannelId}
      onClose={closeViewerPanel}
      onMessageClick={handleViewerMessageClick}
    />
  {/if}

  <!-- Auto-scroll indicator -->
  {#if !autoScroll && chatStore.filteredMessages.length > 0}
    <button
      onclick={() => {
        autoScroll = true;
        if (chatContainer) {
          chatContainer.scrollTop = chatContainer.scrollHeight;
        }
      }}
      class="absolute bottom-4 right-4 px-4 py-2 text-white rounded-full shadow-lg transition-colors"
      style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);"
    >
      Scroll to bottom
    </button>
  {/if}
</div>
