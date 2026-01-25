<script lang="ts">
  interface Props {
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    isDeleting?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let {
    title,
    message,
    confirmText = '削除',
    cancelText = 'キャンセル',
    isDeleting = false,
    onConfirm,
    onCancel
  }: Props = $props();
</script>

<!-- Modal backdrop -->
<div
  class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
  onclick={(e) => {
    if (e.target === e.currentTarget && !isDeleting) onCancel();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape' && !isDeleting) onCancel();
  }}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <!-- Modal content -->
  <div class="bg-[var(--bg-white)] rounded-xl shadow-2xl w-full max-w-md mx-4 overflow-hidden border border-[var(--border-light)]">
    <!-- Header -->
    <div class="px-6 py-4 bg-red-50 border-b border-red-200">
      <div class="flex items-center gap-3">
        <div class="p-2 bg-red-100 rounded-full">
          <svg class="w-6 h-6 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        </div>
        <h2 class="text-xl font-semibold text-[var(--text-primary)]">{title}</h2>
      </div>
    </div>

    <!-- Body -->
    <div class="p-6">
      <p class="text-[var(--text-secondary)]">{message}</p>
    </div>

    <!-- Footer -->
    <div class="px-6 py-4 bg-[var(--bg-light)] border-t border-[var(--border-light)] flex justify-end gap-3">
      <button
        onclick={onCancel}
        disabled={isDeleting}
        class="px-4 py-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors disabled:opacity-50"
      >
        {cancelText}
      </button>
      <button
        onclick={onConfirm}
        disabled={isDeleting}
        class="px-6 py-2 bg-red-500 text-white font-semibold rounded-lg hover:bg-red-600 transition-colors disabled:opacity-50"
      >
        {isDeleting ? '削除中...' : confirmText}
      </button>
    </div>
  </div>
</div>
