<script lang="ts">
  import { chatStore } from '$lib/stores';
  import ChatMessageComponent from './ChatMessage.svelte';
  import { ViewerInfoPanel } from '$lib/components/viewer';
  import type { ChatMessage } from '$lib/types';

  let chatContainer: HTMLDivElement;

  // Auto-scroll is now controlled by chatStore (synced with FilterPanel)
  let autoScrollEnabled = $derived(chatStore.autoScroll);

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

  // Debounce timer for auto-scroll
  let scrollDebounceTimeout: ReturnType<typeof setTimeout> | null = null;

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
  // Debounced to prevent UI freeze from rapid message arrivals
  $effect(() => {
    const messages = chatStore.filteredMessages;
    // Skip auto-scroll if suppressed or disabled by checkbox
    if (suppressAutoScroll || !autoScrollEnabled || !chatContainer || messages.length === 0) {
      return;
    }

    // Debounce: only scroll 50ms after the last message arrives
    // The clearTimeout ensures only the last scheduled scroll executes
    if (scrollDebounceTimeout) {
      clearTimeout(scrollDebounceTimeout);
    }
    scrollDebounceTimeout = setTimeout(() => {
      scrollToBottom();
      scrollDebounceTimeout = null;
    }, 50);
  });

  // Respond to scrollToLatest trigger from FilterPanel
  $effect(() => {
    const trigger = chatStore.scrollToLatestTrigger;
    if (trigger > 0) {
      scrollToBottom();
    }
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
    chatStore.setAutoScroll(false);
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

<div class="flex flex-col h-full bg-[var(--bg-white)] relative">
  <!-- Messages (no header - controls are in FilterPanel) -->
  <div
    bind:this={chatContainer}
    class="flex-1 overflow-y-auto p-3 space-y-2"
    style="font-size: {chatStore.messageFontSize}px;"
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
</div>
