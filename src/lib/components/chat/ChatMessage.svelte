<script lang="ts">
  /**
   * コメントの 1 行 (docs/specs/02_chat.md「配信中の画面」「メッセージ表示仕様」、見た目の値は docs/design/tokens.md)
   *
   * - 強調は優先順位 (src/lib/design/priority.ts) で決める。1: 面 / 2: 薄い背景 + 塗りの印 / 3: 枠の印 / 4: 点 / 5: 印なし
   * - 優先順位 2〜5 は 2 段: 1 段目に名前・#N・時刻を小さく、2 段目に本文 (本文の頭がどの行もそろい、斜め読みで視線が横に振られない。
   *   試作 src/routes/design-proto/scan の A)。どの段も省略せず折り返す (省略された文字を読むには操作が要る)
   * - text-chat-meta の中に text-chat-meta を重ねない (em が二重にかかって縮む)
   * - 相手の配信のコメントは、強さを変えずに配信者名の札で見分ける
   * - メモが登録されている視聴者は、本文の下に「メモ」の札とメモを毎回出す (試作 src/routes/design-proto/memo の B)
   * - 文字の大きさは表示の設定 (fontSize) を行の font-size にし、中の文字は em の段階 (text-chat-*) で決める
   * - 試作は src/routes/design-proto/Row.svelte
   */
  import type { ChatMessage } from '$lib/types';
  import { formatTimestamp } from '$lib/utils/format';
  import { messagePriority, type MessageOrigin } from '$lib/design/priority';
  import { paidLabelOf } from '$lib/design/paid-label';

  interface Props {
    message: ChatMessage;
    fontSize: number;
    showTimestamps: boolean;
    highlighted?: boolean;
    onClick?: () => void;
    /** 主の配信か相手の配信か (docs/specs/02_chat.md「主の配信と相手の配信」) */
    origin?: MessageOrigin;
    /** パネルで登録し直した読み仮名。undefined なら届いたときの値 (author_reading) を使う。空文字は「読み仮名なし」 */
    reading?: string;
    /** 詰めた形 (流れたものの欄・スパチャ等の一覧の詳細)。アイコンを出さず、優先順位 1 を面にせず 1 行にする */
    compact?: boolean;
    /** パネル・視聴者のタブで登録し直したメモ。undefined なら届いたときの値 (author_notes) を使う。空文字は「メモなし」 */
    notes?: string;
  }

  let {
    message,
    fontSize,
    showTimestamps,
    highlighted = false,
    onClick,
    origin = 'primary',
    reading,
    compact = false,
    notes,
  }: Props = $props();

  let priority = $derived(messagePriority(message, origin));
  let partner = $derived(origin === 'partner');
  let colors = $derived(message.metadata?.superchat_colors ?? null);
  let paidLabel = $derived(paidLabelOf(message));
  let authorReading = $derived(reading ?? message.author_reading ?? '');
  let authorNotes = $derived(notes ?? message.author_notes ?? '');
  let formattedTime = $derived(formatTimestamp(message.timestamp));
  let isGift = $derived(message.message_type === 'gift');
  let isSystem = $derived(message.message_type === 'system');
  // ステッカーの content は代わりの文字 ([Sticker]) なので本文として出さない
  let hasBody = $derived(message.message_type !== 'supersticker' && (message.content !== '' || message.runs.length > 0));

  // 画像が読めなければ、アイコンは頭文字に、バッジは消す (壊れた画像の印を出さない)
  let iconFailed = $state(false);
  let iconUrl = $derived(iconFailed ? null : message.author_icon_url);
  let brokenBadges = $state<readonly string[]>([]);

  // バッジ: YouTube の画像があればそれ、メンバーで画像が無ければ「メンバー」の札
  let badgeImages = $derived(
    (message.metadata?.badge_info ?? []).filter((b) => b.image_url && !brokenBadges.includes(b.image_url)),
  );
  let showMemberText = $derived(message.is_member && badgeImages.length === 0);

  // メンバーの名前は緑。ただし優先順位 2 の薄い背景の上では読みにくいので通常の色 (tokens.md「強調」)
  let nameClass = $derived(message.is_member && priority !== 2 ? 'text-member-text' : '');

  // ギフト (ジュエル): 画像が読めなければ文だけにする
  let giftName = $derived(message.metadata?.gift_name ?? message.content);
  let jewelCount = $derived(message.metadata?.jewel_count ?? null);
  let giftImageFailed = $state(false);
  let giftImageUrl = $derived(giftImageFailed ? null : (message.metadata?.gift_image_url ?? null));

  let isMembership = $derived(message.message_type === 'membership' || message.message_type === 'membership_gift');

  /** 面 (優先順位 1) の種類。詰めた形では面にせず 1 行 (line) にする */
  let surface = $derived.by(() => {
    if (priority !== 1) return null;
    if (compact) return 'line';
    if (isGift) return 'gift';
    if (isMembership) return 'member';
    return colors ? 'tier' : 'plain';
  });
</script>

{#snippet face(cls: string)}
  {#if compact}
    <!-- 詰めた形ではアイコンを出さない -->
  {:else if iconUrl}
    <img src={iconUrl} alt="" class="mt-px size-chat-icon shrink-0 rounded-full" onerror={() => (iconFailed = true)} />
  {:else}
    <span
      class="mt-px inline-flex size-chat-icon shrink-0 items-center justify-center rounded-full text-chat-meta font-bold {cls}"
      aria-hidden="true">{[...message.author][0]?.toUpperCase() ?? '?'}</span
    >
  {/if}
{/snippet}

{#snippet runs()}
  {#if message.runs && message.runs.length > 0}
    {#each message.runs as run, i (i)}
      {#if run.type === 'Text'}{run.content}{:else if run.type === 'Emoji'}<img
          src={run.image_url}
          alt={run.alt_text}
          title={run.alt_text}
          class="mx-0.5 inline-block h-chat-emoji w-auto align-middle"
        />{/if}
    {/each}
  {:else}
    {message.content}
  {/if}
{/snippet}

{#snippet badges()}
  {#each badgeImages as badge, i (i)}
    <img
      src={badge.image_url}
      alt={badge.tooltip}
      title={badge.tooltip}
      class="ml-1 inline-block size-chat-badge align-middle"
      onerror={() => (brokenBadges = [...brokenBadges, badge.image_url ?? ''])}
    />
  {/each}
  {#if showMemberText}
    <span class="ml-1 rounded-control border border-member-text px-1 text-chat-meta whitespace-nowrap text-member-text">メンバー</span>
  {/if}
  {#if message.metadata?.is_moderator}<span class="ml-1 text-chat-meta" title="モデレーター">🔧</span>{/if}
  {#if message.metadata?.is_verified}<span class="ml-1 text-chat-meta" title="認証済み">✓</span>{/if}
{/snippet}

{#snippet meta(tone: string)}
  {#if message.in_stream_comment_count != null}
    <span class="ml-1.5 text-chat-meta whitespace-nowrap tabular-nums {tone}">#{message.in_stream_comment_count}</span>
  {/if}
  {#if showTimestamps}
    <span data-testid="message-time" class="ml-1 text-chat-meta whitespace-nowrap tabular-nums {tone}">{formattedTime}</span>
  {/if}
{/snippet}

<!-- 名前 (読み仮名)。size は名前の大きさ (2 段の行では text-chat-meta。読み仮名は兄弟にして em を重ねない) -->
{#snippet name(size = '')}
  <span data-testid="author-name" class="font-bold break-words {size} {nameClass}">{message.author}</span>
  {#if authorReading}
    <span class="ml-1 text-chat-meta text-fg-2">({authorReading})</span>
  {/if}
{/snippet}

<!-- メモ: 本文の下の 1 行。札の大きさは囲む文字に合わせる (text-chat-meta を重ねると em が二重にかかって縮む) -->
{#snippet notesLine(cls: string)}
  <p data-testid="author-notes" class="text-chat-meta break-words {cls}">
    <span class="rounded-control border border-current px-1 whitespace-nowrap">メモ</span>
    {authorNotes}
  </p>
{/snippet}

<div
  class="cursor-pointer text-chat-body text-fg-1 {highlighted ? 'outline-2 -outline-offset-2 outline-accent' : ''}"
  style="font-size: {fontSize}px;"
  data-message-id={message.id}
  data-priority={priority}
  data-origin={origin}
  onclick={onClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === 'Enter' && onClick?.()}
>
  {#if surface === 'line'}
    <!-- 優先順位 1 を 1 行に詰めたもの: 時刻・札 (金額・メンバーシップの種類)・名前・本文。お礼で順番に読み上げるので時刻は表示の設定によらず出す -->
    <div class="flex items-baseline gap-2 px-3 py-1.5 hover:bg-fg-1/8">
      <span class="shrink-0 text-chat-meta whitespace-nowrap text-fg-3 tabular-nums">{formattedTime}</span>
      <span
        data-testid="paid-chip"
        class="shrink-0 rounded-control px-1.5 text-chat-meta font-bold whitespace-nowrap tabular-nums {colors
          ? ''
          : isMembership
            ? 'bg-member text-on-member'
            : 'border border-outline'}"
        style={colors ? `background-color: ${colors.header_background}; color: ${colors.header_text};` : undefined}>{paidLabel}</span
      >
      <div class="min-w-0">
        <p class="break-words">
          {@render name()}
          {#if isGift}{#if jewelCount != null}<span class="ml-1.5">{giftName} を送りました</span>{/if}{:else if hasBody}<span
              class="ml-1.5">{@render runs()}</span
            >{/if}
        </p>
        {#if authorNotes}{@render notesLine('mt-0.5 text-fg-2')}{/if}
      </div>
    </div>
  {:else if surface === 'tier' && colors}
    <!-- スパチャ・ステッカー: YouTube のティアの色の面 (浮いて見える理由がある数少ない要素なので角丸) -->
    <div class="mx-2 my-1 overflow-hidden rounded-control" style="background: {colors.body_background};">
      <div class="flex items-center gap-2 px-3 py-1.5" style="background: {colors.header_background}; color: {colors.header_text};">
        {@render face('bg-surface/30')}
        <span class="min-w-0 font-bold break-words"
          >{message.author}{#if authorReading}<span class="ml-1 text-chat-meta font-normal">({authorReading})</span>{/if}</span
        >
        <span class="shrink-0 font-normal">{@render meta('')}</span>
        <span class="ml-auto shrink-0 text-chat-amount tabular-nums">{paidLabel}</span>
      </div>
      {#if hasBody}
        <p data-testid="paid-body" class="px-3 pt-1 pb-2 break-words" style="color: {colors.body_text};">{@render runs()}</p>
      {/if}
      {#if authorNotes}<div class="px-3 pb-2" style="color: {colors.body_text};">{@render notesLine('')}</div>{/if}
    </div>
  {:else if surface === 'member' || surface === 'plain'}
    <!-- メンバーシップ・メンバーシップギフト・マイルストーン (member の面)。色の無いスパチャは中立の面 -->
    <div
      class="mx-2 my-1 overflow-hidden rounded-control {surface === 'member'
        ? 'bg-member text-on-member'
        : 'border border-outline bg-surface-highest'}"
    >
      <div class="flex items-center gap-2 px-3 py-1.5">
        {@render face('bg-surface/30')}
        <span class="min-w-0 font-bold break-words"
          >{message.author}{#if authorReading}<span class="ml-1 text-chat-meta font-normal">({authorReading})</span>{/if}</span
        >
        <span class="shrink-0 font-normal">{@render meta('')}</span>
        <span class="ml-auto shrink-0 font-bold {surface === 'member' ? 'text-chat-meta' : 'text-chat-amount tabular-nums'}">{paidLabel}</span>
      </div>
      {#if hasBody}
        <p data-testid="paid-body" class="px-3 pt-0 pb-2 break-words">{@render runs()}</p>
      {/if}
      {#if authorNotes}<div class="px-3 pb-2">{@render notesLine('')}</div>{/if}
    </div>
  {:else if surface === 'gift'}
    <!-- ギフト (ジュエル) -->
    <div class="mx-2 my-1 overflow-hidden rounded-control border border-outline bg-surface-highest">
      <div class="flex items-center gap-2 px-3 py-1.5">
        {#if giftImageUrl}
          <img
            data-testid="gift-image"
            src={giftImageUrl}
            alt={giftName}
            class="size-chat-gift shrink-0"
            onerror={() => (giftImageFailed = true)}
          />
        {/if}
        <span class="min-w-0 break-words">{message.author} が {giftName} を送りました{@render meta('text-fg-2')}</span>
        {#if jewelCount != null}
          <span data-testid="gift-jewels" class="ml-auto shrink-0 text-chat-amount font-bold tabular-nums">{jewelCount} ジュエル</span>
        {/if}
      </div>
    </div>
  {:else if isSystem}
    <p class="px-3 py-1.5 break-words text-fg-2">{@render runs()}</p>
  {:else}
    <!-- 優先順位 2〜5: 1 段目に名前の行、2 段目に本文。本文の頭はアイコンの右でそろう。
         相手の配信なら名前の行の上に配信元の行を足す (アイコンは名前の行の横のまま。空きの幅でアイコンの右にそろえる) -->
    <div class="px-3 py-1 hover:bg-fg-1/8 {priority === 2 ? 'bg-highlight/20' : ''}">
      {#if partner}
        <div class="flex gap-2">
          {#if !compact}<span class="w-chat-icon shrink-0" aria-hidden="true"></span>{/if}
          <p data-testid="source-label" class="min-w-0 flex-1 text-chat-meta break-words text-fg-2">{message.broadcaster_name}</p>
        </div>
      {/if}
      <div class="flex items-start gap-2">
        {@render face('bg-surface-highest text-fg-2')}
        <div class="min-w-0 flex-1">
          <p data-testid="name-line" class="break-words">
            {#if priority === 2}
              <span class="mr-1 rounded-control bg-highlight px-1 text-chat-meta font-bold text-surface">初見</span>
            {:else if priority === 3}
              <span class="mr-1 rounded-control border border-highlight px-1 text-chat-meta text-highlight">初見</span>
            {:else if priority === 4}
              <span class="mr-0.5 text-chat-meta text-highlight" aria-label="配信内で初めてのコメント">●</span>
            {/if}
            {@render name('text-chat-meta')}
            {@render badges()}
            {#if paidLabel}
              <span data-testid="paid-label" class="ml-1 rounded-control border border-outline px-1 text-chat-meta whitespace-nowrap tabular-nums"
                >{paidLabel}</span
              >
            {/if}
            {@render meta(priority === 2 ? 'text-fg-2' : 'text-fg-3')}
          </p>
          {#if hasBody}<p data-testid="message-body" class="break-words">{@render runs()}</p>{/if}
          {#if authorNotes}{@render notesLine('mt-0.5 text-fg-2')}{/if}
        </div>
      </div>
    </div>
  {/if}
</div>
