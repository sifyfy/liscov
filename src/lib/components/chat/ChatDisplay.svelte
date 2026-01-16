<script lang="ts">
  import { chatStore } from '$lib/stores';
  import ChatMessageComponent from './ChatMessage.svelte';
  import { ViewerInfoPanel } from '$lib/components/viewer';
  import type { ChatMessage } from '$lib/types';

  let chatContainer: HTMLDivElement;

  // Auto-scroll setting (checkbox controlled, same as original liscov)
  // This is the ONLY control for auto-scroll behavior
  let autoScrollEnabled = $state(true);

  // Flag to temporarily suppress auto-scroll during programmatic scrolling
  let suppressAutoScroll = $state(false);

  // Selected viewer for ViewerInfoPanel
  let selectedViewer = $state<{
    channelId: string;
    displayName: string;
    iconUrl?: string;
    message: ChatMessage;
  } | null>(null);

  // Highlighted message ID (for scroll-to feature)
  let highlightedMessageId = $state<string | null>(null);

  // Reliable scroll to bottom with retry
  function scrollToBottom() {
    if (!chatContainer) return;

    // Use multiple attempts to ensure scroll completes after DOM update
    const doScroll = () => {
      if (chatContainer) {
        chatContainer.scrollTop = chatContainer.scrollHeight;
      }
    };

    // First attempt: immediate
    doScroll();

    // Second attempt: after next frame (DOM update)
    requestAnimationFrame(() => {
      doScroll();
      // Third attempt: after a short delay for images/dynamic content
      setTimeout(doScroll, 50);
    });
  }

  // Auto-scroll when new messages arrive (controlled by checkbox only)
  $effect(() => {
    const messages = chatStore.filteredMessages;
    // Skip auto-scroll if suppressed or disabled by checkbox
    if (suppressAutoScroll || !autoScrollEnabled || !chatContainer || messages.length === 0) {
      return;
    }
    scrollToBottom();
  });

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

    // Disable auto-scroll (same as original liscov)
    autoScrollEnabled = false;
    suppressAutoScroll = true;

    // Highlight the message first (so it's visible when scrolled to)
    highlightedMessageId = message.id;

    // Use requestAnimationFrame to ensure DOM is updated before scrolling
    requestAnimationFrame(() => {
      // Scroll to the message in main chat
      const targetElement = chatContainer?.querySelector(`[data-message-id="${message.id}"]`);
      if (targetElement) {
        targetElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }

      // Re-enable auto-scroll suppression check after scroll animation completes
      setTimeout(() => {
        suppressAutoScroll = false;
      }, 500);

      // Clear highlight after 3 seconds
      setTimeout(() => {
        highlightedMessageId = null;
      }, 3000);
    });
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
      <!-- Auto-scroll toggle (same as original liscov) -->
      <label class="flex items-center gap-1 text-xs text-[var(--text-muted)] cursor-pointer">
        <input
          type="checkbox"
          checked={autoScrollEnabled}
          onchange={(e) => autoScrollEnabled = (e.target as HTMLInputElement).checked}
          class="w-3 h-3"
        />
        自動スクロール
      </label>
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
        <ChatMessageComponent
          {message}
          highlighted={highlightedMessageId === message.id}
          onClick={() => handleMessageClick(message)}
        />
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

  <!-- Scroll to bottom button (shown when auto-scroll is disabled) -->
  {#if !autoScrollEnabled && chatStore.filteredMessages.length > 0}
    <button
      onclick={() => {
        autoScrollEnabled = true;
        scrollToBottom();
      }}
      class="absolute bottom-4 right-4 px-4 py-2 text-white rounded-full shadow-lg transition-colors"
      style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);"
    >
      最新に戻る
    </button>
  {/if}
</div>
