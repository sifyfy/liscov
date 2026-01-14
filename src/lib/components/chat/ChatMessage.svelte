<script lang="ts">
  import type { ChatMessage, MessageRun } from '$lib/types';
  import { chatStore } from '$lib/stores';

  interface Props {
    message: ChatMessage;
    onClick?: () => void;
  }

  let { message, onClick }: Props = $props();
  let fontSize = $derived(chatStore.messageFontSize);
  let showTimestamps = $derived(chatStore.showTimestamps);

  // Get SuperChat colors from metadata or use defaults
  let superchatColors = $derived(() => {
    if (message.metadata?.superchat_colors) {
      return message.metadata.superchat_colors;
    }
    return null;
  });

  // Determine container styles based on message type and actual YouTube colors
  let containerStyle = $derived(() => {
    const baseStyle = 'bg-[var(--bg-card)] border border-[var(--border-light)] rounded-lg shadow-sm';
    const colors = superchatColors();

    switch (message.message_type) {
      case 'superchat':
        if (colors) {
          return `${baseStyle} border-l-4`;
        }
        return `${baseStyle} border-l-4 border-l-yellow-500 bg-yellow-50`;
      case 'supersticker':
        if (colors) {
          return `${baseStyle} border-l-4`;
        }
        return `${baseStyle} border-l-4 border-l-orange-500 bg-orange-50`;
      case 'membership':
        return `${baseStyle} border-l-4 border-l-[var(--member-border)] bg-[var(--member-bg)]`;
      case 'membership_gift':
        return `${baseStyle} border-l-4 border-l-emerald-500 bg-emerald-50`;
      case 'system':
        return `${baseStyle} border-l-4 border-l-blue-500 bg-blue-50`;
      default:
        return baseStyle;
    }
  });

  // Dynamic inline style for SuperChat actual colors
  let dynamicStyle = $derived(() => {
    const colors = superchatColors();
    if (colors && (message.message_type === 'superchat' || message.message_type === 'supersticker')) {
      return `border-left-color: ${colors.header_background}; background: linear-gradient(135deg, ${colors.body_background}22 0%, ${colors.header_background}22 100%);`;
    }
    return '';
  });

  // Format timestamp to HH:MM:SS
  let formattedTime = $derived(() => {
    // timestamp is already in HH:MM:SS format from backend
    if (message.timestamp && message.timestamp.includes(':')) {
      return message.timestamp;
    }
    try {
      const date = new Date(message.timestamp);
      if (isNaN(date.getTime())) {
        return message.timestamp;
      }
      return date.toLocaleTimeString('ja-JP', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
      });
    } catch {
      return message.timestamp;
    }
  });

  // Message type header text
  let typeHeader = $derived(() => {
    switch (message.message_type) {
      case 'superchat':
        return 'Super Chat';
      case 'supersticker':
        return 'Super Sticker';
      case 'membership':
        return 'New Member';
      case 'membership_gift':
        return 'Membership Gift';
      default:
        return null;
    }
  });

  // Get header color from YouTube colors or default
  let headerColor = $derived(() => {
    const colors = superchatColors();
    if (colors && (message.message_type === 'superchat' || message.message_type === 'supersticker')) {
      return colors.header_background;
    }
    switch (message.message_type) {
      case 'superchat':
        return '#fbbf24'; // yellow-500
      case 'supersticker':
        return '#f97316'; // orange-500
      case 'membership':
        return '#22c55e'; // green-500
      case 'membership_gift':
        return '#10b981'; // emerald-500
      default:
        return null;
    }
  });

  // Comment count display
  let commentCountDisplay = $derived(() => {
    if (message.comment_count === null || message.comment_count === undefined) {
      return null;
    }
    if (message.comment_count === 1) {
      return '🎉#1';
    }
    return `#${message.comment_count}`;
  });

  // Check if message has emoji runs
  function hasEmoji(runs: MessageRun[]): boolean {
    return runs.some(run => run.type === 'Emoji');
  }
</script>

<div
  class="px-3 py-2 cursor-pointer hover:ring-2 hover:ring-[var(--primary-start)]/30 transition-all {containerStyle()}"
  style={dynamicStyle()}
  onclick={onClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === 'Enter' && onClick?.()}
>
  <!-- Type header for special messages -->
  {#if typeHeader()}
    <div class="mb-1.5">
      <span
        class="text-xs font-medium px-2 py-0.5 rounded-full"
        style={headerColor() ? `background-color: ${headerColor()}; color: white;` : ''}
      >
        {typeHeader()}
        {#if message.amount}
          <span class="ml-1 font-bold">{message.amount}</span>
        {/if}
      </span>
    </div>
  {/if}

  <!-- Row 1: Metadata (icon, name, badges, comment count, timestamp) -->
  <div class="flex items-center gap-2" style="font-size: {fontSize}px;">
    <!-- Author icon -->
    {#if message.author_icon_url}
      <img
        src={message.author_icon_url}
        alt=""
        class="w-6 h-6 rounded-full flex-shrink-0"
      />
    {:else}
      <div
        class="w-6 h-6 rounded-full flex items-center justify-center flex-shrink-0 text-white text-xs font-bold"
        style="background: linear-gradient(135deg, var(--primary-start) 0%, var(--primary-end) 100%);"
      >
        {message.author.charAt(0).toUpperCase()}
      </div>
    {/if}

    <!-- Author name -->
    <span class="font-medium text-[var(--text-primary)] truncate max-w-[200px]">
      {message.author}
    </span>

    <!-- Badge images from metadata -->
    {#if message.metadata?.badge_info}
      {#each message.metadata.badge_info as badge}
        {#if badge.image_url}
          <img
            src={badge.image_url}
            alt={badge.tooltip}
            title={badge.tooltip}
            class="w-4 h-4 flex-shrink-0"
          />
        {/if}
      {/each}
    {/if}

    <!-- Moderator badge -->
    {#if message.metadata?.is_moderator}
      <span class="px-1 py-0.5 text-xs bg-blue-100 text-blue-700 rounded border border-blue-300 font-medium" title="モデレーター">
        🔧
      </span>
    {/if}

    <!-- Verified badge -->
    {#if message.metadata?.is_verified}
      <span class="px-1 py-0.5 text-xs bg-gray-100 text-gray-700 rounded border border-gray-300 font-medium" title="認証済み">
        ✓
      </span>
    {/if}

    <!-- Member badge (only if no badge_info image) -->
    {#if message.is_member && (!message.metadata?.badge_info || message.metadata.badge_info.every(b => !b.image_url))}
      <span class="px-1.5 py-0.5 text-xs bg-green-100 text-green-700 rounded border border-green-300 font-medium">
        Member
      </span>
    {/if}

    <!-- Comment count -->
    {#if commentCountDisplay()}
      <span class="text-xs {message.comment_count === 1 ? 'text-orange-600 font-bold' : 'text-[var(--text-muted)]'}">
        {commentCountDisplay()}
      </span>
    {/if}

    <!-- Amount badge for SuperChat (when not shown in header) -->
    {#if message.amount && !typeHeader()}
      <span class="px-1.5 py-0.5 text-xs bg-yellow-100 text-yellow-800 rounded border border-yellow-300 font-bold">
        {message.amount}
      </span>
    {/if}

    <!-- Timestamp -->
    {#if showTimestamps}
      <span class="text-xs text-[var(--text-muted)] ml-auto flex-shrink-0">
        {formattedTime()}
      </span>
    {/if}
  </div>

  <!-- Row 2: Message content with runs (text + emoji) -->
  <div class="mt-1 ml-8">
    <p class="text-[var(--text-secondary)] break-words leading-relaxed" style="font-size: {fontSize}px;">
      {#if message.runs && message.runs.length > 0}
        {#each message.runs as run, index}
          {#if run.type === 'Text'}
            <span>{run.content}</span>
          {:else if run.type === 'Emoji'}
            <img
              src={run.image_url}
              alt={run.alt_text}
              title={run.alt_text}
              class="inline-block align-middle mx-0.5"
              style="height: {fontSize + 4}px; width: auto;"
            />
          {/if}
        {/each}
      {:else}
        {message.content}
      {/if}
    </p>
  </div>
</div>
