<script lang="ts">
  import { chatStore } from '$lib/stores';
  import { VList, type VListHandle } from 'virtua/svelte';
  import ChatMessageComponent from './ChatMessage.svelte';
  import { ViewerInfoPanel } from '$lib/components/viewer';
  import type { ChatMessage } from '$lib/types';

  let vlist = $state<VListHandle | undefined>();

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

  // Props passed to ChatMessage (avoid per-component $derived)
  let fontSize = $derived(chatStore.messageFontSize);
  let showTimestamps = $derived(chatStore.showTimestamps);

  // Auto-scroll when new messages arrive
  $effect(() => {
    const msgs = chatStore.displayedMessages;
    if (suppressAutoScroll || !autoScrollEnabled || !vlist || msgs.length === 0) {
      return;
    }
    // Use queueMicrotask to scroll after virtua processes the new data
    queueMicrotask(() => {
      vlist?.scrollToIndex(msgs.length - 1, { align: 'end' });
    });
  });

  // Respond to scrollToLatest trigger from FilterPanel (fire only on trigger change)
  let prevScrollTrigger = 0;
  $effect(() => {
    const trigger = chatStore.scrollToLatestTrigger;
    if (trigger === prevScrollTrigger || !vlist) return;
    prevScrollTrigger = trigger;
    queueMicrotask(() => {
      const msgs = chatStore.displayedMessages;
      if (msgs.length > 0) {
        vlist?.scrollToIndex(msgs.length - 1, { align: 'end' });
      }
    });
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

    // Highlight the message
    highlightedMessageId = message.id;

    // Find index in displayedMessages and scroll to it
    const msgs = chatStore.displayedMessages;
    const targetIndex = msgs.findIndex((m) => m.id === message.id);
    if (targetIndex !== -1 && vlist) {
      vlist.scrollToIndex(targetIndex, { align: 'center' });
    }

    // Re-enable auto-scroll suppression check after scroll animation completes
    setTimeout(() => {
      suppressAutoScroll = false;
    }, 500);

    // Clear highlight after 3 seconds
    setTimeout(() => {
      highlightedMessageId = null;
    }, 3000);
  }
</script>

<div class="flex flex-col h-full bg-[var(--bg-surface-1)] relative">
  <!-- Messages -->
  {#if chatStore.displayedMessages.length === 0}
    <div class="flex-1 flex items-center justify-center p-3">
      <p class="text-[var(--text-muted)] text-center">
        {#if chatStore.isConnected}
          Waiting for messages...
        {:else}
          Connect to a stream to see chat messages
        {/if}
      </p>
    </div>
  {:else}
    <VList
      bind:this={vlist}
      data={chatStore.displayedMessages}
      getKey={(item) => item.id}
      style="flex: 1; overflow-y: auto; padding: 12px; font-size: {fontSize}px;"
    >
      {#snippet children(message)}
        {@const showSource = chatStore.connections.size >= 2}
        {@const conn = chatStore.connections.get(Number(message.connection_id))}
        <div class="mb-1">
          <ChatMessageComponent
            {message}
            {fontSize}
            {showTimestamps}
            highlighted={highlightedMessageId === message.id}
            showSourceIndicator={showSource}
            sourceColor={conn?.color}
            sourceName={conn?.broadcasterName}
            onClick={() => handleMessageClick(message)}
          />
        </div>
      {/snippet}
    </VList>
  {/if}

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
