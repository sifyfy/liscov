<script lang="ts">
  /**
   * 「最新へ」のボタン (docs/specs/02_chat.md「配信中の画面」の「自動スクロール」)
   *
   * 最新から離れているときだけ、流れの右下に出す。自動で最新に戻るまでの残り時間を、ボタンの下端の帯が縮む動きで表す。
   * 帯は期限が決まったときの残り時間から 0 まで縮む。期限が延びたら帯だけを作り直して最初からやり直す
   * (ボタンごと作り直すと、マウスを載せて期限が延びるたびにボタンが入れ替わって押せなくなる)。
   * 動きを減らす設定 (prefers-reduced-motion) では帯を出さず、残りの秒数を文字で出す。
   * 視聴者情報パネルで打っている間 (期限なし) は戻らないので、帯も秒数も出さない。
   * 離れている間に届いた優先順位 1・2 の件数を添える (「最新へ スパチャ等 2・新規さん 1」。0 件の種類は出さない)
   */
  import { untrack } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { AUTO_RETURN_MS } from '$lib/utils/auto-return';

  interface Props {
    /** 最新に戻る時刻 (ミリ秒)。null は戻らない (視聴者情報パネルで打っている間) */
    deadline: number | null;
    /** いまの時刻 (ミリ秒) */
    now: number;
    /** 離れている間に届いた優先順位 1・2 の件数 */
    unread: Readonly<{ supporters: number; newcomers: number }>;
    onClick: () => void;
  }

  let { deadline, now, unread, onClick }: Props = $props();

  let unreadLabel = $derived(
    [
      unread.supporters > 0 ? `スパチャ等 ${unread.supporters}` : null,
      unread.newcomers > 0 ? `新規さん ${unread.newcomers}` : null,
    ]
      .filter((part) => part !== null)
      .join('・'),
  );

  // 帯が縮みはじめるときの残り時間。期限が変わったときだけ測る (時刻が進むたびに変えると帯の動きが跳ぶ)。
  // 期限を一度 $derived で受けるのは、props がまとめて差し替えられても (値が同じなら) 測り直さないため
  let currentDeadline = $derived(deadline);
  let barMs = $derived(currentDeadline === null ? 0 : Math.max(0, currentDeadline - untrack(() => now)));
  let seconds = $derived(deadline === null ? 0 : Math.ceil(Math.max(0, deadline - now) / 1000));
</script>

<button
  type="button"
  onclick={onClick}
  class="relative flex items-center gap-1 overflow-hidden rounded-full bg-surface-high py-1.5 pr-3 pl-2.5 text-label text-fg-1 shadow-elevation-2 hover:bg-surface-highest"
>
  <Icon name="scroll-down" size={14} />最新へ{#if unreadLabel}<span class="text-fg-2 tabular-nums">{` ${unreadLabel}`}</span>{/if}
  {#if deadline !== null}
    <span data-testid="latest-remaining" class="hidden text-caption text-fg-2 motion-reduce:inline">あと {seconds} 秒で戻ります</span>
  {/if}
  {#key deadline}
    {#if deadline !== null}
      <!-- eslint-disable better-tailwindcss/no-unknown-classes -- latest-bar は下の <style> で定義した縮む動き -->
      <span
        data-testid="latest-bar"
        class="latest-bar absolute bottom-0 left-0 h-0.5 w-full origin-left bg-accent motion-reduce:hidden"
        style:--from={barMs / AUTO_RETURN_MS}
        style:animation-duration="{barMs}ms"
        aria-hidden="true"
      ></span>
      <!-- eslint-enable better-tailwindcss/no-unknown-classes -->
    {/if}
  {/key}
</button>

<style>
  .latest-bar {
    animation-name: latest-shrink;
    animation-timing-function: linear;
    animation-fill-mode: forwards;
  }
  @keyframes latest-shrink {
    from {
      transform: scaleX(var(--from));
    }
    to {
      transform: scaleX(0);
    }
  }
</style>
