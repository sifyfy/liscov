<script lang="ts">
  /**
   * 視聴者情報パネル (docs/specs/02_chat.md「配信中の画面」の「視聴者の読み仮名・メモ」と「視聴者情報パネル」)
   *
   * 押したコメントの行のすぐ下に広げる (置き場所は ChatDisplay が決める)。試作は src/routes/design-proto/viewer の C。
   * - 読み込みが終わりしだい読み仮名の欄にフォーカスする。読み仮名・メモの欄で Enter を押すと保存し (変換の確定は除く)、onSaved で閉じてもらう
   * - Esc・× は保存せずに閉じる (onClose)
   * - 欄にフォーカスがある・保存していない変更がある間は「打っている」(onBusyChange)。その間は流れが最新に戻らない
   */
  import { invoke } from '@tauri-apps/api/core';
  import type { ChatMessage } from '$lib/types';
  import { chatStore } from '$lib/stores';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { paidLabelOf } from '$lib/design/paid-label';
  import { formatTimestamp } from '$lib/utils/format';
  import { messageKey } from '$lib/utils/message-key';

  interface Props {
    viewer: {
      channelId: string;
      displayName: string;
      iconUrl?: string;
      /** 押したコメント (この人のコメントの中で印を付ける) */
      message: ChatMessage;
    };
    /** 保存先の配信者 (押したコメントが届いた接続の配信者) */
    broadcasterChannelId: string;
    /** 接続が 2 つ以上のときの保存先の配信者名。1 つなら null (札を出さない) */
    broadcasterName: string | null;
    /** この人のコメント (届いた順) */
    history: readonly ChatMessage[];
    onClose: () => void;
    /** 保存できた。呼び出し側が閉じる */
    onSaved: () => void;
    /** 打っているか (欄にフォーカスがある・保存していない変更がある) が変わった */
    onBusyChange?: (busy: boolean) => void;
    /** この人のコメントの行を押した */
    onMessageClick?: (message: ChatMessage) => void;
  }

  let { viewer, broadcasterChannelId, broadcasterName, history, onClose, onSaved, onBusyChange, onMessageClick }: Props =
    $props();

  let reading = $state('');
  let notes = $state('');
  // 読み込んだ値 (保存していない変更があるかの比べる先)
  let loaded = $state({ reading: '', notes: '' });
  let isSaving = $state(false);
  let error = $state('');
  let viewerProfileId = $state<number | null>(null);
  // 読み込みが終わるまで欄と保存を押せなくする (読み込み結果が入力中の内容を上書きしないように)
  let isLoaded = $state(false);
  let focusWithin = $state(false);
  let historyOpen = $state(false);
  let readingInput = $state<HTMLInputElement>();
  let root = $state<HTMLElement>();

  let dirty = $derived(isLoaded && (reading !== loaded.reading || notes !== loaded.notes));
  let busy = $derived(focusWithin || dirty);
  let clickedKey = $derived(messageKey(viewer.message));
  let newestFirst = $derived([...history].reverse());

  $effect(() => {
    onBusyChange?.(busy);
  });

  // 視聴者が変わったら読み込み直す。読み込み中に別の視聴者を開いたら、前の結果は捨てる
  // (古い応答が後から届いて、別の視聴者のフォームと保存先を上書きしないように)
  $effect(() => {
    const bc = broadcasterChannelId;
    const vc = viewer.channelId;
    let stale = false;

    reading = '';
    notes = '';
    loaded = { reading: '', notes: '' };
    error = '';
    viewerProfileId = null;
    isLoaded = false;

    loadCustomInfo(bc, vc)
      .then((info) => {
        if (stale || info === null) return;
        viewerProfileId = info.profileId;
        reading = info.reading;
        notes = info.notes;
        loaded = { reading: info.reading, notes: info.notes };
      })
      .catch((e) => console.error('Failed to load viewer info:', e))
      .finally(() => {
        if (!stale) isLoaded = true;
      });

    return () => {
      stale = true;
    };
  });

  // 読み込みが終わったら読み仮名の欄にすぐ打てるようにする
  $effect(() => {
    if (isLoaded) readingInput?.focus();
  });

  async function loadCustomInfo(bc: string, vc: string) {
    const profile = await invoke<{ id: number } | null>('viewer_get_profile', { broadcasterId: bc, channelId: vc });
    if (!profile) return null;
    const customInfo = await invoke<{ reading: string | null; notes: string | null } | null>('viewer_get_custom_info', {
      viewerProfileId: profile.id,
    });
    return { profileId: profile.id, reading: customInfo?.reading ?? '', notes: customInfo?.notes ?? '' };
  }

  async function save() {
    if (!isLoaded || isSaving) return;
    if (viewerProfileId === null) {
      error = '視聴者プロファイルが見つかりません';
      return;
    }
    isSaving = true;
    error = '';
    try {
      await invoke('viewer_upsert_custom_info', {
        viewerProfileId,
        reading: reading || null,
        notes: notes || null,
        customData: null,
      });
      // 画面にある同じ視聴者の行にも、すぐ読み仮名とメモを出す (02_chat.md「配信中の画面」1 行の形)
      chatStore.setViewerInfo(broadcasterChannelId, viewer.channelId, { reading, notes });
      onSaved();
    } catch (e) {
      console.error('Failed to save viewer info:', e);
      error = '保存に失敗しました';
    } finally {
      isSaving = false;
    }
  }

  /** 欄で Enter → 保存 (日本語入力の変換を確定する Enter は除く) */
  function handleFieldKeydown(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing || e.keyCode === 229) return;
    e.preventDefault();
    void save();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    onClose();
  }

  function handleFocusOut(e: FocusEvent) {
    if (!root?.contains(e.relatedTarget as Node | null)) focusWithin = false;
  }
</script>

<!-- Esc とフォーカスの出入りは、中の欄・ボタンから伝わってくるものをまとめて受ける (section 自体は操作しない) -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
  bind:this={root}
  aria-label="視聴者情報"
  data-viewer-panel
  class="mx-2 my-1 flex flex-col gap-2 rounded-control border border-accent/40 bg-surface-low p-2 text-fg-1"
  onkeydown={handleKeydown}
  onfocusin={() => (focusWithin = true)}
  onfocusout={handleFocusOut}
>
  <!-- 誰の情報か -->
  <div class="flex items-center gap-2">
    {#if viewer.iconUrl}
      <img src={viewer.iconUrl} alt="" class="size-6 shrink-0 rounded-full" />
    {:else}
      <span
        class="flex size-6 shrink-0 items-center justify-center rounded-full bg-surface-highest text-caption font-bold text-fg-2"
        aria-hidden="true">{viewer.displayName.slice(0, 1)}</span
      >
    {/if}
    <span class="min-w-0 flex-1 text-label font-bold break-words">{viewer.displayName}</span>
    {#if broadcasterName}
      <span
        data-testid="viewer-broadcaster"
        title="この配信者のもとに保存します"
        class="shrink-0 rounded-control border border-outline px-1.5 text-caption whitespace-nowrap text-fg-2"
      >
        保存先 {broadcasterName}
      </span>
    {/if}
    <button
      type="button"
      aria-label="閉じる"
      title="閉じる (Esc)"
      onclick={onClose}
      class="flex size-7 shrink-0 items-center justify-center rounded-control text-fg-2 hover:bg-fg-1/8"
    >
      <Icon name="close" size={14} />
    </button>
  </div>

  <!-- 読み仮名・メモ・保存 -->
  <div class="flex flex-wrap gap-2">
    <label class="flex min-w-48 flex-1 flex-col gap-1 text-caption text-fg-2">
      読み仮名
      <input
        bind:this={readingInput}
        bind:value={reading}
        type="text"
        placeholder="例: やまだ たろう"
        disabled={!isLoaded}
        onkeydown={handleFieldKeydown}
        class="rounded-control border border-outline bg-surface px-2 py-1 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent disabled:opacity-50"
      />
    </label>
    <label class="flex min-w-48 flex-1 flex-col gap-1 text-caption text-fg-2">
      メモ
      <input
        bind:value={notes}
        type="text"
        placeholder="例: 攻略情報をよくくれる"
        disabled={!isLoaded}
        onkeydown={handleFieldKeydown}
        class="rounded-control border border-outline bg-surface px-2 py-1 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent disabled:opacity-50"
      />
    </label>
  </div>
  <div class="flex flex-wrap items-center gap-2">
    <button
      type="button"
      onclick={save}
      disabled={!isLoaded || isSaving}
      class="rounded-control bg-accent px-3 py-1 text-label text-surface hover:opacity-90 disabled:opacity-50"
    >
      保存
    </button>
    <span class="text-caption text-fg-2">Enter で保存</span>
    {#if error}
      <p role="alert" class="text-caption text-error">{error}</p>
    {/if}
  </div>

  <!-- この人のコメント (畳んである) -->
  <button
    type="button"
    aria-expanded={historyOpen}
    onclick={() => (historyOpen = !historyOpen)}
    class="flex items-center gap-1 self-start rounded-control text-caption text-accent hover:underline"
  >
    <Icon name="chevron-down" size={12} class={historyOpen ? 'rotate-180' : ''} />
    この人のコメント <span class="tabular-nums">{history.length}</span> 件
  </button>
  {#if historyOpen}
    <ul class="max-h-48 overflow-y-auto">
      {#each newestFirst as message (messageKey(message))}
        {@const paid = paidLabelOf(message)}
        <li>
          <button
            type="button"
            data-testid="viewer-history-item"
            onclick={() => onMessageClick?.(message)}
            class="w-full rounded-control px-1.5 py-0.5 text-left text-body break-words hover:bg-fg-1/8 {messageKey(message) ===
            clickedKey
              ? 'bg-accent/15'
              : ''}"
          >
            <span class="mr-1.5 text-caption text-fg-2 tabular-nums">{formatTimestamp(message.timestamp)}</span>
            {#if paid}<span class="mr-1.5 text-caption font-bold tabular-nums">{paid}</span>{/if}{message.content}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>
