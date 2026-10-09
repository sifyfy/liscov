<script lang="ts">
  /**
   * 「最新へ」のボタン (docs/specs/02_chat.md「配信中の画面」の「自動スクロール」)
   *
   * 最新から離れているときだけ、流れの右下に出す。自動で最新に戻るまでの残り時間を、ボタンの下端の帯が縮む動きで表す。
   * 帯は期限が決まったときの残り時間から 0 まで縮む。期限が延びたら帯だけを作り直して最初からやり直す
   * (ボタンごと作り直すと、マウスを載せて期限が延びるたびにボタンが入れ替わって押せなくなる)。
   * 動きを減らす設定 (prefers-reduced-motion) では帯を出さず、残りの秒数を文字で出す
   */
  import { untrack } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { AUTO_RETURN_MS } from '$lib/utils/auto-return';

  interface Props {
    /** 最新に戻る時刻 (ミリ秒) */
    deadline: number;
    /** いまの時刻 (ミリ秒) */
    now: number;
    onClick: () => void;
  }

  let { deadline, now, onClick }: Props = $props();

  // 帯が縮みはじめるときの残り時間。期限が変わったときだけ測る (時刻が進むたびに変えると帯の動きが跳ぶ)。
  // 期限を一度 $derived で受けるのは、props がまとめて差し替えられても (値が同じなら) 測り直さないため
  let currentDeadline = $derived(deadline);
  let barMs = $derived(Math.max(0, currentDeadline - untrack(() => now)));
  let seconds = $derived(Math.ceil(Math.max(0, deadline - now) / 1000));
</script>

<button
  type="button"
  onclick={onClick}
  class="relative flex items-center gap-1 overflow-hidden rounded-full bg-surface-high py-1.5 pr-3 pl-2.5 text-label text-fg-1 shadow-elevation-2 hover:bg-surface-highest"
>
  <Icon name="scroll-down" size={14} />最新へ
  <span data-testid="latest-remaining" class="hidden text-caption text-fg-2 motion-reduce:inline">あと {seconds} 秒で戻ります</span>
  {#key deadline}
    <span
      data-testid="latest-bar"
      class="latest-bar absolute bottom-0 left-0 h-0.5 w-full origin-left bg-accent motion-reduce:hidden"
      style:--from={barMs / AUTO_RETURN_MS}
      style:animation-duration="{barMs}ms"
      aria-hidden="true"
    ></span>
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
