<script lang="ts">
  import type { ChatMessage } from '$lib/types';
  import { formatTimestamp } from '$lib/utils/format';

  interface Props {
    message: ChatMessage;
    fontSize: number;
    showTimestamps: boolean;
    highlighted?: boolean;
    onClick?: () => void;
    // 配信元インジケーター（多接続時に使用）
    showSourceIndicator?: boolean;
    sourceColor?: string;
    sourceName?: string;
  }

  let { message, fontSize, showTimestamps, highlighted = false, onClick, showSourceIndicator = false, sourceColor, sourceName }: Props = $props();

  // SuperChat の配色（metadata に含まれる場合のみ）
  let superchatColors = $derived(message.metadata?.superchat_colors ?? null);

  // メッセージ種別ごとの動的インラインスタイル（liscov オリジナルの左枠スタイル）
  let dynamicStyle = $derived.by(() => {
    const colors = superchatColors;
    if (colors && (message.message_type === 'superchat' || message.message_type === 'supersticker')) {
      return `border-left-color: ${colors.body_background}; background: linear-gradient(135deg, ${colors.header_background}33 0%, ${colors.body_background} 100%);`;
    }

    switch (message.message_type) {
      case 'superchat':
        return 'border-left-color: #f6ad55; background: var(--bg-surface-2);';
      case 'supersticker':
        return 'border-left-color: #fc8181; background: var(--bg-surface-2);';
      case 'membership':
        // Check if milestone
        if (message.metadata?.milestone_months) {
          return 'border-left-color: #9f7aea; background: var(--bg-surface-2);';
        }
        return 'border-left-color: var(--member-accent); background: var(--member-subtle);';
      case 'membership_gift':
        return 'border-left-color: #4299e1; background: var(--info-subtle);';
      case 'gift':
        return 'border-left-color: var(--accent); background: var(--accent-subtle);';
      case 'system':
        return 'border-left-color: #4299e1; background: var(--info-subtle);';
      default:
        // Normal text or member
        if (message.is_member) {
          return 'border-left-color: var(--member-accent); background: var(--member-subtle);';
        }
        return 'border-left-color: var(--accent); background: var(--bg-surface-2);';
    }
  });

  // タイムスタンプ（ローカルタイムゾーンの HH:MM:SS）
  let formattedTime = $derived(formatTimestamp(message.timestamp));

  // メッセージ種別のヘッダーテキスト
  let typeHeader = $derived.by(() => {
    switch (message.message_type) {
      case 'superchat':
        return 'スーパーチャット';
      case 'supersticker':
        return 'スーパーステッカー';
      case 'membership':
        return '新規メンバー';
      case 'membership_gift':
        return 'メンバーシップギフト';
      default:
        return null;
    }
  });

  // ヘッダー色（YouTube の配色があればそれを使い、なければ種別ごとの既定色）
  let headerColor = $derived.by(() => {
    const colors = superchatColors;
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

  // ギフト（ジュエル）の表示内容。画像が読めなければ文だけにする
  let isGift = $derived(message.message_type === 'gift');
  let giftName = $derived(message.metadata?.gift_name ?? message.content);
  let jewelCount = $derived(message.metadata?.jewel_count ?? null);
  let giftImageFailed = $state(false);
  let giftImageUrl = $derived(giftImageFailed ? null : (message.metadata?.gift_image_url ?? null));

  // 初見さん判定
  let isFirstTimeViewer = $derived(message.is_first_time_viewer);

  // 配信内コメント回数表示
  let commentCountDisplay = $derived(
    message.in_stream_comment_count == null ? null : `#${message.in_stream_comment_count}`
  );

</script>

<div
  class="px-3 py-2 cursor-pointer hover:ring-2 hover:ring-[var(--accent)]/30 transition-all rounded border-l-4"
  style="{dynamicStyle}{highlighted ? 'border: 2px solid var(--accent); box-shadow: 0 0 8px var(--accent-subtle);' : ''}"
  data-message-id={message.id}
  onclick={onClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === 'Enter' && onClick?.()}
>
  <!-- 配信元インジケーター（多接続時：2接続以上のとき表示） -->
  {#if showSourceIndicator && sourceColor}
    <div class="source-indicator-row">
      <div class="source-indicator" style="background-color: {sourceColor}"></div>
      <span class="source-label">{sourceName ?? ''}</span>
    </div>
  {/if}

  <!-- Type header for special messages -->
  {#if typeHeader}
    {#if superchatColors}
      <div
        class="-mx-3 -mt-2 mb-1.5 px-3 py-1 flex items-center justify-between rounded-tr"
        style="background-color: {headerColor}; color: {superchatColors.header_text};"
      >
        <span class="text-xs font-semibold tracking-wide">{typeHeader}</span>
        {#if message.amount}
          <span class="text-xs font-bold">{message.amount}</span>
        {/if}
      </div>
    {:else}
      <div class="mb-1.5">
        <span
          class="text-xs font-medium px-2 py-0.5 rounded-full"
          style={headerColor ? `background-color: ${headerColor}; color: white;` : ''}
        >
          {typeHeader}
          {#if message.amount}
            <span class="ml-1 font-bold">{message.amount}</span>
          {/if}
        </span>
      </div>
    {/if}
  {/if}

  <!-- Row 1: Metadata (icon, name, badges, comment count, timestamp) -->
  <div class="flex items-center gap-2 {superchatColors ? 'bg-[var(--bg-surface-2)]/80 -mx-1 px-1 py-0.5 rounded-md' : ''}" style="font-size: {fontSize}px;">
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
        style="background: var(--accent);"
      >
        {message.author.charAt(0).toUpperCase()}
      </div>
    {/if}

    <!-- Author name (member=green, non-member=blue) -->
    <!-- min-w-0 で flex の縮小ポイントを名前に集約し、利用可能幅まで表示。
         バッジ・タイムスタンプ(flex-shrink-0)は常に表示を維持し、
         1行に収まらない場合のみ名前を末尾省略(…)する -->
    <span
      class="font-medium truncate min-w-0"
      style="color: {message.is_member ? 'var(--member-accent)' : 'var(--accent)'};"
    >
      {message.author}
    </span>

    <!-- Badge images from metadata -->
    {#if message.metadata?.badge_info}
      {#each message.metadata.badge_info as badge, i (i)}
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
      <span class="px-1 py-0.5 text-xs bg-[var(--info-subtle)] text-[var(--info)] rounded border border-[var(--border-default)] font-medium" title="モデレーター">
        🔧
      </span>
    {/if}

    <!-- Verified badge -->
    {#if message.metadata?.is_verified}
      <span class="px-1 py-0.5 text-xs bg-[var(--bg-surface-3)] text-[var(--text-secondary)] rounded border border-[var(--border-default)] font-medium" title="認証済み">
        ✓
      </span>
    {/if}

    <!-- Member badge (only if no badge_info image) -->
    {#if message.is_member && (!message.metadata?.badge_info || message.metadata.badge_info.every(b => !b.image_url))}
      <span class="px-1.5 py-0.5 text-xs bg-[var(--member-subtle)] text-[var(--member-accent)] rounded border border-[var(--border-default)] font-medium">
        メンバー
      </span>
    {/if}

    <!-- 初見さんバッジ (初コメ時は目立つ、2回目以降はmuted) -->
    {#if isFirstTimeViewer}
      {#if message.in_stream_comment_count === 1}
        <span class="px-1.5 py-0.5 bg-[var(--success-subtle)] text-[var(--success)] rounded font-bold" style="font-size: {fontSize}px;">
          🎉初見さん
        </span>
      {:else}
        <span class="text-[var(--text-muted)]" style="font-size: {fontSize}px;">
          初見さん
        </span>
      {/if}
    {/if}

    <!-- 配信内コメント回数 (#1は目立つ色、#2以降はmuted) -->
    {#if commentCountDisplay}
      <span class="{message.in_stream_comment_count === 1 ? 'font-bold text-[var(--warning)]' : 'text-[var(--text-muted)]'}" style="font-size: {fontSize}px;">
        {commentCountDisplay}
      </span>
    {/if}

    <!-- Amount badge for SuperChat (when not shown in header) -->
    {#if message.amount && !typeHeader}
      <span class="px-1.5 py-0.5 text-xs bg-[var(--warning-subtle)] text-[var(--warning)] rounded border border-[var(--border-default)] font-bold">
        {message.amount}
      </span>
    {/if}

    <!-- Timestamp -->
    {#if showTimestamps}
      <span class="text-xs text-[var(--text-muted)] ml-auto flex-shrink-0">
        {formattedTime}
      </span>
    {/if}
  </div>

  <!-- Row 2: Message content with runs (text + emoji) -->
  {#if isGift}
    <div class="mt-1 ml-8 flex items-center gap-2" style="font-size: {fontSize}px;">
      {#if giftImageUrl}
        <img
          data-testid="gift-image"
          src={giftImageUrl}
          alt={giftName}
          class="flex-shrink-0"
          style="width: 32px; height: 32px;"
          onerror={() => (giftImageFailed = true)}
        />
      {/if}
      <span class="break-words leading-relaxed text-[var(--text-secondary)]">
        {message.author} が {giftName} を送りました
      </span>
      {#if jewelCount != null}
        <span
          data-testid="gift-jewels"
          class="px-1.5 py-0.5 text-xs bg-[var(--warning-subtle)] text-[var(--warning)] rounded border border-[var(--border-default)] font-bold flex-shrink-0"
        >
          {jewelCount} ジュエル
        </span>
      {/if}
    </div>
  {:else}
  <div class="mt-1 ml-8">
    <p class="break-words leading-relaxed" style="font-size: {fontSize}px; color: {superchatColors && (message.message_type === 'superchat' || message.message_type === 'supersticker') ? superchatColors.body_text : 'var(--text-secondary)'};">
      {#if message.runs && message.runs.length > 0}
        {#each message.runs as run, i (i)}
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
  {/if}
</div>

<style>
  /* 配信元インジケーター（多接続時に使用） */
  .source-indicator-row {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-bottom: 4px;
  }
  .source-indicator {
    width: 3px;
    height: 12px;
    border-radius: 1px;
    flex-shrink: 0;
  }
  .source-label {
    font-size: 0.7em;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
