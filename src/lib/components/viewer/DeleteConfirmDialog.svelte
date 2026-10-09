<script lang="ts">
  /**
   * 取り消せない削除の確認ダイアログ (docs/specs/06_viewer.md「視聴者のタブ」の「この配信者の記録を消す…」)
   *
   * 開いたらフォーカスは「やめる」(Enter の押し間違いで消さない)。Esc・外側を押してもやめる (消している間は除く)
   */
  interface Props {
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    isDeleting?: boolean;
    /** 失敗したときの文。あればダイアログの中に出す */
    error?: string | null;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let {
    title,
    message,
    confirmText = '消す',
    cancelText = 'やめる',
    isDeleting = false,
    error = null,
    onConfirm,
    onCancel
  }: Props = $props();

  const focusOnOpen = (node: HTMLElement) => node.focus();
</script>

<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-surface/80 p-4"
  onclick={(e) => {
    if (e.target === e.currentTarget && !isDeleting) onCancel();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape' && !isDeleting) onCancel();
  }}
  role="dialog"
  aria-modal="true"
  aria-labelledby="delete-confirm-title"
  tabindex="-1"
>
  <div class="flex w-full max-w-md flex-col gap-3 rounded-control border border-error bg-surface-high p-4 text-fg-1">
    <h2 id="delete-confirm-title" class="text-title">{title}</h2>
    <p class="text-body">{message}</p>
    {#if error}
      <p role="alert" class="text-caption text-error">{error}</p>
    {/if}
    <div class="flex justify-end gap-2">
      <button
        type="button"
        onclick={onCancel}
        disabled={isDeleting}
        {@attach focusOnOpen}
        class="rounded-control border border-outline px-3 py-1 text-label text-fg-2 hover:bg-fg-1/8 disabled:opacity-50"
      >
        {cancelText}
      </button>
      <button
        type="button"
        onclick={onConfirm}
        disabled={isDeleting}
        class="rounded-control bg-error px-3 py-1 text-label text-surface hover:opacity-90 disabled:opacity-50"
      >
        {isDeleting ? '消しています…' : confirmText}
      </button>
    </div>
  </div>
</div>
