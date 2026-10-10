<script lang="ts">
  /**
   * 斜め読みしやすいコメントの行 (/design-proto/scan)
   *
   * 名前と本文を 1 行に続ける形は、名前の長さで本文の書き出しの位置が行ごとに左右にずれ、
   * 本文を縦にたどる斜め読みで視線が横に振られた (シフィさん、2026-10-10)。
   * 4 案 (A 名前の行 + 本文 / B 名前の列 / C 1 行 / D 名前の列 右寄せ) を比べ、A を選んだ (02_chat.md「通常のコメント」)。
   * B・D は長い名前の人が頻繁にコメントすると、毎回名前の列の中で折り返して行が伸びるので採らなかった。
   * 選んだ後は本体の ChatMessage が A の形で出すので、ここでは本体の部品で見せる。4 案の試作のコードはこのファイルの履歴にある
   * 各行に data-scan-row (本文) を付け、撮影で本文の書き出しの位置を測る (e2e/design-capture.capture.ts「斜め読みの試作」)
   */
  import ChatMessage from '$lib/components/chat/ChatMessage.svelte';
  import { feedOf, originOf, priorityOf, stickToBottom } from '../data';

  // 登録してある読み仮名とメモ (架空。メモの試作と同じ)
  const NOTES: Readonly<Record<string, string>> = {
    夜ふかし研究会: '攻略情報をよくくれる。ホラーが苦手',
    しおこんぶ: 'ネタバレしがち。注意',
    ねむいペンギン: '赤スパの人。衣装の感想をくれる',
    のりたま: '9/20 のサムネのイラストを描いてくれた。お礼を言う',
    みかん箱: '箱推し',
  };
  const READINGS: Readonly<Record<string, string>> = { 夜ふかし研究会: 'よふかし', のりたま: 'のりたま' };
  const feed = feedOf('normal').map((m) => ({ ...m, author_reading: READINGS[m.author] ?? null, author_notes: NOTES[m.author] ?? null }));
</script>

<main class="flex h-screen flex-col bg-surface font-ui text-fg-1">
  <div class="min-h-0 flex-1 overflow-y-auto" data-scan-list {@attach stickToBottom}>
    {#each feed as m (m.id)}
      {@const plain = priorityOf(m) !== 1 && m.message_type === 'text' && m.content !== ''}
      <div data-scan-row={plain ? m.content : undefined}>
        <ChatMessage message={m} fontSize={13} showTimestamps origin={originOf(m)} />
      </div>
    {/each}
  </div>
</main>
