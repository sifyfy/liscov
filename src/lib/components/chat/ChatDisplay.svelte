<script lang="ts">
  /**
   * 配信中の画面のコメントの部分 (docs/specs/02_chat.md「配信中の画面」「メッセージ表示仕様」)
   *
   * 上から: コメントの流れ / 流れた新規さん / この配信のスパチャ等の一覧 (概要・詳細を切り替える)
   * - 流れたもの = 流れの画面のいちばん上に見えている行より前に届いた優先順位 1・2 (src/lib/design/flow.ts)
   * - スパチャ等の一覧の概要は流れたもの、詳細は時刻順の全件 (配信の最後のお礼)。起動・F5 では概要
   * - 流れの高さが変わったとき (欄の伸び縮み・概要と詳細の切り替え・ウィンドウの大きさ) も、自動スクロール中なら最新を見せ続ける
   * - 自動スクロール: 手でスクロールして最新から離れたら止め、流れの上で 30 秒操作が無ければ最新に戻る (src/lib/utils/auto-return.ts)。
   *   離れている間は右下に「最新へ」のボタン (離れている間に届いた優先順位 1・2 の件数を添える)
   * - 視聴者情報パネル: 押したコメントの行のすぐ下に広げる (「視聴者の読み仮名・メモ」)。流れは止まり、打っている間は最新に戻らない。
   *   閉じたら、開く前に最新を追っていれば最新に戻る
   */
  import { tick as nextTick, untrack } from 'svelte';
  import { chatStore, configStore } from '$lib/stores';
  import { VList, type VListHandle } from 'virtua/svelte';
  import ChatMessageComponent from './ChatMessage.svelte';
  import FlowedSection from './FlowedSection.svelte';
  import { arrivedSince, flowedAbove } from '$lib/design/flow';
  import { FLOWED_HEIGHT } from '$lib/design/flowed-height';
  import { ViewerInfoPanel } from '$lib/components/viewer';
  import type { ChatMessage } from '$lib/types';
  import { messageKey, type MessageKey } from '$lib/utils/message-key';
  import {
    FOLLOWING,
    activity,
    hold,
    leaveLatest,
    reachLatest,
    release,
    tick,
    type AutoReturnState,
  } from '$lib/utils/auto-return';
  import LatestButton from './LatestButton.svelte';

  let vlist = $state<VListHandle | undefined>();

  // 自動スクロール (最新を追っているか、離れていつ戻るか)
  let autoReturn = $state<AutoReturnState>(FOLLOWING);
  let following = $derived(autoReturn.following);
  // いまの時刻 (離れている間だけ 1 秒ごとに進める。「最新へ」の残りの秒数と期限の確かめに使う)
  let now = $state(Date.now());

  // 手でスクロールしたか (ホイール・ドラッグ・キー・タッチの直後のスクロールだけを手によるものとみなす。
  // virtua が行の高さを測り直してスクロール位置を変えたときに、止めないため)
  const USER_INPUT_WINDOW_MS = 1000;
  let lastUserInput = 0;
  // 視聴者情報パネルの中での入力・クリックは数えない (打っている・押しているだけで、流れをスクロールしようとはしていない)
  const markUserInput = (e: Event) => {
    if (e.target instanceof Element && e.target.closest('[data-viewer-panel]')) return;
    lastUserInput = Date.now();
  };

  // 最新から離れたときの一覧の最後のもの (離れている間に届いた優先順位 1・2 を数える起点)
  let awaySince = $state.raw<ChatMessage | undefined>(undefined);
  const NO_ARRIVALS = { supporters: 0, newcomers: 0 } as const;
  let arrivals = $derived(
    !following && awaySince ? arrivedSince(chatStore.displayedMessages, awaySince, chatStore.originOf) : NO_ARRIVALS,
  );

  /** 自動スクロールの状態を、いまの時刻で変える */
  function updateAutoReturn(change: (state: AutoReturnState, at: number) => AutoReturnState) {
    now = Date.now();
    const next = change(autoReturn, now);
    if (next === autoReturn) return;
    if (autoReturn.following && !next.following) awaySince = chatStore.displayedMessages.at(-1);
    autoReturn = next;
  }

  // 最新とみなす、下端からの距離 (px)
  const LATEST_SLACK_PX = 4;

  function handleScroll() {
    updateFirstVisible();
    if (!vlist) return;
    const atLatest = vlist.getScrollOffset() + vlist.getViewportSize() >= vlist.getScrollSize() - LATEST_SLACK_PX;
    const byHand = Date.now() - lastUserInput < USER_INPUT_WINDOW_MS;
    if (atLatest) {
      // 視聴者情報パネルを開いている間は、手で最新まで戻ったときだけ追うのを再開する
      // (最新のコメントの下にパネルを広げると下端に着くが、それで追いはじめるとパネルが流れていく)
      if (!autoReturn.following && (!panel || byHand)) updateAutoReturn(reachLatest);
    } else if (byHand) {
      // virtua の scrollToIndex は、行の高さが測れるまで目標の位置へ合わせ直し続ける。
      // 手で離れたら、今の位置への scrollBy(0) で置き換えて、最新へ引き戻されないようにする
      vlist.scrollBy(0);
      updateAutoReturn(leaveLatest);
    }
  }

  /**
   * 「最新へ」を押した。押したときの入力 (pointerdown・keydown) を手でのスクロールと数えないよう記録を消す
   * (ボタンが消えて流れが伸びるときのスクロールで、また離れたとみなさないため)
   */
  function goLatest() {
    lastUserInput = 0;
    updateAutoReturn(reachLatest);
  }

  /** 流れの上での操作 (クリック・マウスの移動)。離れている間は戻るまでの時間を延ばす */
  function handleActivity() {
    updateAutoReturn(activity);
  }

  // 離れている間は 1 秒ごとに期限を確かめ、過ぎたら最新に戻る (視聴者情報パネルも閉じる)
  $effect(() => {
    if (following) return;
    const timer = setInterval(() => {
      now = Date.now();
      autoReturn = tick(autoReturn, now);
      if (autoReturn.following && panel) closePanel();
    }, 1000);
    return () => clearInterval(timer);
  });

  function scrollToLatest() {
    const msgs = chatStore.displayedMessages;
    if (vlist && msgs.length > 0) vlist.scrollToIndex(msgs.length - 1, { align: 'end' });
  }

  /**
   * 視聴者情報パネル (開いていなければ null)。
   * - broadcasterChannelId は押したコメントが届いた接続の配信者。パネル内のコメントを押しても変えない (02_chat.md「どの配信者のもとで開くか」)
   * - anchorKey と place は広げる行と、その行がある所 (流れ・流れたものの欄。スパチャ等の一覧の詳細も流れたものの欄)
   * - wasFollowing は開く前に最新を追っていたか (閉じたら最新に戻す)。returnFocus は閉じたらフォーカスを戻す行
   */
  type PanelPlace = 'flow' | 'flowed';
  let panel = $state<{
    viewer: { channelId: string; displayName: string; iconUrl?: string; message: ChatMessage };
    broadcasterChannelId: string;
    broadcasterName: string | null;
    anchorKey: MessageKey;
    place: PanelPlace;
    wasFollowing: boolean;
    returnFocus: HTMLElement | null;
  } | null>(null);
  // パネルで打っているか (欄にフォーカスがある・保存していない変更がある)。打っている間は最新に戻らない
  let panelBusy = $state(false);
  // 読み上げ用の知らせ (「保存しました」)。パネルは保存すると閉じるので、外に置く
  let announcement = $state('');

  // Highlighted message ID (for scroll-to feature)
  let highlightedMessageKey = $state<MessageKey | null>(null);

  // この配信のスパチャ等の一覧の表示。概要 = 流れたもの、詳細 = 時刻順の全件 (配信の最後のお礼)。覚えない
  type SupportersView = 'summary' | 'detail';
  let supportersView = $state<SupportersView>('summary');

  // 流れの画面のいちばん上に見えている行 (一部だけ見えているものを含む)。何も無ければ undefined
  let firstVisible = $state.raw<ChatMessage | undefined>(undefined);

  function updateFirstVisible() {
    const msgs = chatStore.displayedMessages;
    const next = vlist && msgs.length > 0 ? msgs[vlist.findItemIndex(vlist.getScrollOffset())] : undefined;
    if (next !== firstVisible) firstVisible = next;
  }

  // 一覧が変わったら (届いた・クリア・フィルタ) 見えている行を測り直す
  $effect(() => {
    void chatStore.displayedMessages;
    queueMicrotask(updateFirstVisible);
  });

  // 流れたもの。フィルタで隠しているものは流れていないので、フィルタ後の一覧の中で前にあるものだけ
  let flowedSupporters = $derived(flowedAbove(chatStore.supporters, chatStore.filteredMessages, firstVisible));
  let flowedNewcomers = $derived(flowedAbove(chatStore.newcomers, chatStore.filteredMessages, firstVisible));
  // スパチャ等の一覧の中身 (詳細はフィルタ・表示件数によらない)
  let supportersItems = $derived(supportersView === 'detail' ? chatStore.supporters : flowedSupporters);

  /** 流れの高さが変わったら、自動スクロール中なら最新に合わせ直す ({@attach}) */
  function followResize(node: HTMLElement): () => void {
    const observer = new ResizeObserver(() => {
      const msgs = chatStore.displayedMessages;
      if (following && vlist && msgs.length > 0) {
        vlist.scrollToIndex(msgs.length - 1, { align: 'end' });
      }
      updateFirstVisible();
    });
    observer.observe(node);
    return () => observer.disconnect();
  }

  // Props passed to ChatMessage (avoid per-component $derived)
  let fontSize = $derived(chatStore.messageFontSize);
  let showTimestamps = $derived(chatStore.showTimestamps);

  // コメントが無くなったら (クリア・再接続) 離れる先も無いので、最新を追う状態に戻す
  $effect(() => {
    if (chatStore.displayedMessages.length === 0) untrack(() => updateAutoReturn(reachLatest));
  });

  // 打っている間は最新に戻らない。打ち終えたら (フォーカスが外れ変更も無い)、そこから 30 秒で戻る。
  // 追っている間は何もしない (追っていて、また離れたときにも当てはめ直すため following にも反応する)
  $effect(() => {
    const busy = panelBusy;
    void following;
    untrack(() => updateAutoReturn(busy ? hold : release));
  });

  // パネルを広げた行がある一覧
  function placeItems(place: PanelPlace): readonly ChatMessage[] {
    if (place === 'flow') return chatStore.displayedMessages;
    return [...flowedNewcomers, ...supportersItems];
  }

  // 流れの中でパネルを広げた行の位置 (virtua が画面の外に出ても消さないように keepMounted に渡す)
  let panelIndex = $derived(
    panel?.place === 'flow' ? chatStore.displayedMessages.findLastIndex((m) => messageKey(m) === panel?.anchorKey) : -1,
  );

  // 広げた行が一覧から無くなったら (コメントを消す・表示件数で外れた・流れたものの欄から外れた) 閉じる
  $effect(() => {
    if (!panel) return;
    const { place, anchorKey } = panel;
    const exists = place === 'flow' ? panelIndex !== -1 : placeItems(place).some((m) => messageKey(m) === anchorKey);
    if (!exists) untrack(closePanel);
  });

  // 最新を追っている間は、新着が来たら (と、戻ったとき) 最新までスクロールする。virtua が新しい行を受け取ってから動かす
  $effect(() => {
    const msgs = chatStore.displayedMessages;
    if (!following || !vlist || msgs.length === 0) return;
    queueMicrotask(scrollToLatest);
  });

  /**
   * コメントを押した (from は押した行がある所)。押した行のすぐ下に視聴者情報パネルを広げ、流れを止める。
   * 流れたものの欄 (スパチャ等の一覧の詳細を含む) で押したら、流れのそのコメントまで移ってその下に広げる。流れに無ければ押した行の下に広げる
   */
  function handleMessageClick(message: ChatMessage, from: PanelPlace = 'flow') {
    // 視聴者を特定できないギフト（channel_id が空）は視聴者情報パネルを開かない
    if (!message.channel_id) return;
    // どの配信者のもとの視聴者か分からない（接続が切断済み・配信者不明）ときも開かない
    const broadcasterChannelId = chatStore.broadcasterChannelIdOf(message);
    if (!broadcasterChannelId) return;

    const key = messageKey(message);
    const flowIndex = chatStore.displayedMessages.findLastIndex((m) => messageKey(m) === key);
    const place: PanelPlace = from === 'flow' || flowIndex !== -1 ? 'flow' : from;
    const active = document.activeElement;
    panel = {
      viewer: {
        channelId: message.channel_id,
        displayName: message.author,
        iconUrl: message.author_icon_url || undefined,
        message,
      },
      broadcasterChannelId,
      broadcasterName:
        chatStore.connections.size >= 2
          ? chatStore.connections.get(Number(message.connection_id))?.broadcasterName || null
          : null,
      anchorKey: key,
      place,
      // 開いたまま別のコメントを押したら、最初に開く前の状態を引き継ぐ
      wasFollowing: panel ? panel.wasFollowing : following,
      returnFocus: active instanceof HTMLElement && active !== document.body ? active : null,
    };
    // 押したときの入力を手でのスクロールと数えない (下のスクロールで virtua の合わせ直しを打ち消さないため)
    lastUserInput = 0;
    updateAutoReturn(leaveLatest);
    if (place === 'flow' && flowIndex !== -1) {
      // パネルが描かれて行の高さが変わってから、行とパネルが見える所までスクロールする
      void nextTick().then(() => vlist?.scrollToIndex(flowIndex, { align: 'nearest' }));
    }
  }

  /** パネルを閉じる。開く前に最新を追っていたら最新に戻し、押した行にフォーカスを戻す */
  function closePanel() {
    if (!panel) return;
    const { wasFollowing, returnFocus } = panel;
    panel = null;
    panelBusy = false;
    if (wasFollowing) goLatest();
    if (returnFocus?.isConnected) returnFocus.focus({ preventScroll: true });
  }

  function handlePanelSaved() {
    announcement = '保存しました';
    setTimeout(() => (announcement = ''), 3000);
    closePanel();
  }

  /** パネルの「この人のコメント」の行を押した。流れのそのコメントへ移り 3 秒強調する (パネルは元の行の下のまま) */
  function handleViewerMessageClick(message: ChatMessage) {
    if (panel) panel = { ...panel, viewer: { ...panel.viewer, message } };

    // 過去のコメントに飛ぶので最新から離れる (30 秒操作が無ければ戻る)
    lastUserInput = 0;
    updateAutoReturn(leaveLatest);

    // Highlight the message
    const key = messageKey(message);
    highlightedMessageKey = key;

    // Find index in displayedMessages and scroll to it
    const msgs = chatStore.displayedMessages;
    const targetIndex = msgs.findIndex((m) => messageKey(m) === key);
    if (targetIndex !== -1 && vlist) {
      vlist.scrollToIndex(targetIndex, { align: 'center' });
    }

    // Clear highlight after 3 seconds
    setTimeout(() => {
      highlightedMessageKey = null;
    }, 3000);
  }
</script>

<div class="relative flex h-full flex-col bg-surface">
  <!-- コメントの流れ (最新が下) -->
  <div
    class="relative flex min-h-0 flex-1 flex-col"
    {@attach followResize}
    onwheel={markUserInput}
    onpointerdown={markUserInput}
    onkeydown={markUserInput}
    ontouchstart={markUserInput}
    onpointermove={handleActivity}
    onclick={handleActivity}
    role="presentation"
  >
    {#if chatStore.displayedMessages.length === 0}
      <div class="flex flex-1 items-center justify-center p-3">
        <p class="text-center text-body text-fg-2">
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
        getKey={messageKey}
        style="flex: 1; overflow-y: auto;"
        onscroll={handleScroll}
        keepMounted={panelIndex === -1 ? undefined : [panelIndex]}
      >
        {#snippet children(message)}
          <ChatMessageComponent
            {message}
            {fontSize}
            {showTimestamps}
            origin={chatStore.originOf(message)}
            reading={chatStore.infoOf(message)?.reading}
            notes={chatStore.infoOf(message)?.notes}
            highlighted={highlightedMessageKey === messageKey(message) || panel?.anchorKey === messageKey(message)}
            onClick={() => handleMessageClick(message)}
          />
          {#if panel?.place === 'flow' && panel.anchorKey === messageKey(message)}{@render viewerPanel()}{/if}
        {/snippet}
      </VList>
    {/if}
    {#if !autoReturn.following && chatStore.displayedMessages.length > 0}
      <!-- 「最新へ」はコメントの行に重ねない。離れている間だけ流れの下端に帯をとり、その右に置く (流れは帯の高さぶん縮む) -->
      <div class="flex shrink-0 justify-end px-3 py-1">
        <LatestButton deadline={autoReturn.deadline} {now} unread={arrivals} onClick={goLatest} />
      </div>
    {/if}
  </div>

  <!-- 流れて画面の外に出た優先順位 2・1 (新規さんを上に。欄ごとにスクロール) -->
  <FlowedSection
    label="流れた新規さん"
    items={flowedNewcomers}
    heightRatio={configStore.flowedNewcomersHeightRatio}
    onHeightChange={(ratio) => configStore.setFlowedHeightRatio('newcomers', ratio)}
    {fontSize}
    {showTimestamps}
    originOf={chatStore.originOf}
    infoOf={chatStore.infoOf}
    onClick={(message) => handleMessageClick(message, 'flowed')}
    openKey={panel?.place === 'flowed' ? panel.anchorKey : null}
    panel={viewerPanel}
  />
  <!-- 自分の配信のスパチャ等があれば、流れたものが無くても見出しの 1 行を出す (詳細へ切り替えられるように) -->
  <FlowedSection
    label="この配信のスパチャ等の一覧"
    items={supportersItems}
    layout={supportersView === 'detail' ? 'all' : 'flowed'}
    countLabel={supportersView === 'detail' ? `全 ${supportersItems.length} 件・時刻順` : `流れた ${supportersItems.length}`}
    showEmpty={chatStore.supporters.length > 0}
    actions={supportersViewSwitch}
    maxRatio={FLOWED_HEIGHT.supportersMax}
    heightRatio={configStore.flowedSupportersHeightRatio}
    onHeightChange={(ratio) => configStore.setFlowedHeightRatio('supporters', ratio)}
    {fontSize}
    {showTimestamps}
    originOf={chatStore.originOf}
    infoOf={chatStore.infoOf}
    onClick={(message) => handleMessageClick(message, 'flowed')}
    openKey={panel?.place === 'flowed' ? panel.anchorKey : null}
    panel={viewerPanel}
  />

  <!-- 読み上げ用の知らせ (パネルは保存すると閉じるので外に置く) -->
  <p class="sr-only" aria-live="polite">{announcement}</p>
</div>

<!-- この配信のスパチャ等の一覧の概要・詳細の切り替え (見出しの右端) -->
{#snippet supportersViewSwitch()}
  <div role="group" aria-label="表示" class="flex items-center gap-0.5">
    {#each [['summary', '概要'], ['detail', '詳細']] as const as [view, text] (view)}
      <button
        type="button"
        aria-pressed={supportersView === view}
        onclick={() => (supportersView = view)}
        class="rounded-control px-1.5 text-caption {supportersView === view ? 'bg-accent/15 text-accent' : 'text-fg-2 hover:bg-fg-1/8'}"
      >
        {text}
      </button>
    {/each}
  </div>
{/snippet}

<!-- 視聴者情報パネル。押した行のすぐ下に、流れ・流れたものの欄のどこでも同じものを広げる -->
{#snippet viewerPanel()}
  {#if panel}
    <ViewerInfoPanel
      viewer={panel.viewer}
      broadcasterChannelId={panel.broadcasterChannelId}
      broadcasterName={panel.broadcasterName}
      history={chatStore.getMessagesForChannel(panel.viewer.channelId)}
      onClose={closePanel}
      onSaved={handlePanelSaved}
      onBusyChange={(busy) => (panelBusy = busy)}
      onMessageClick={handleViewerMessageClick}
    />
  {/if}
{/snippet}
