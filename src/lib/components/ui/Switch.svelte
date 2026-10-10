<script lang="ts">
  /**
   * 設定のオン・オフの切り替え。状態は aria-pressed で表す。
   * オンは success の色の溝に面の色のつまみ、オフは控えめな枠だけ (色だけに頼らず、つまみの位置でも分かる)
   */
  interface Props {
    pressed: boolean;
    /** 読み上げ用の名前 (見える文字は呼び出し側が隣に置く) */
    label: string;
    testId?: string;
    onclick: () => void;
  }
  let { pressed, label, testId, onclick }: Props = $props();
</script>

<button
  type="button"
  {onclick}
  data-testid={testId}
  aria-label={label}
  aria-pressed={pressed}
  class="relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors duration-100 ease-standard {pressed
    ? 'border-success bg-success'
    : 'border-outline bg-surface-highest'}"
>
  <span
    class="inline-block size-3 rounded-full transition-transform duration-100 ease-standard {pressed
      ? 'translate-x-5 bg-surface'
      : 'translate-x-1 bg-fg-2'}"
    aria-hidden="true"
  ></span>
</button>
