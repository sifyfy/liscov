<script lang="ts">
  // 描画中の例外をこの領域で止める。1 つの例外で画面全体の更新が止まらないようにする（FEATURE_SPECIFICATION「画面の描画エラー」）
  import type { Snippet } from 'svelte';

  let { name, children }: { name: string; children: Snippet } = $props();
</script>

<svelte:boundary onerror={(error) => console.error(`[${name}] 表示中にエラーが発生しました:`, error)}>
  {@render children()}

  <!-- 例外の中身は画面に出さない（配信に映ることがある）。コンソールにだけ出す -->
  <!-- eslint-disable-next-line @typescript-eslint/no-unused-vars -->
  {#snippet failed(_error, reset)}
    <div class="flex-1 flex flex-col items-center justify-center gap-3 p-6 text-sm text-[var(--text-secondary)]">
      <p>表示中にエラーが発生しました</p>
      <button
        onclick={reset}
        class="px-3 py-1.5 rounded-md bg-[var(--bg-surface-2)] text-[var(--text-primary)] border border-[var(--border-default)] hover:bg-[var(--bg-surface-3)]"
      >再表示</button>
    </div>
  {/snippet}
</svelte:boundary>
