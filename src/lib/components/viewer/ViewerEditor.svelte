<script lang="ts">
  /**
   * 視聴者のタブの編集 (docs/specs/06_viewer.md「視聴者のタブ (配信の後・準備)」)
   *
   * 押した行のすぐ下に広げる (チャットの視聴者情報パネルと同じ作法)。試作は src/routes/design-proto/viewers の C。
   * - 開いたら読み仮名の欄にフォーカスする。読み仮名・タグ・メモの欄で Enter を押すと保存する (変換の確定は除く)
   * - Esc・「やめる」は保存せずに閉じる (onCancel)
   * - 「この視聴者を消す…」は編集の中に確認を出す (フォーカスは「やめる」)
   */
  import { untrack } from 'svelte';
  import type { ViewerWithCustomInfo } from '$lib/types';
  import { formatTags, parseTags } from '$lib/utils/tags';

  interface Props {
    viewer: ViewerWithCustomInfo;
    /** 保存する。呼び出し側が保存して閉じる。失敗したら reject */
    onSave: (values: { reading: string; notes: string; tags: string[] }) => Promise<void>;
    onCancel: () => void;
    /** 消す。失敗したら reject */
    onDelete: () => Promise<void>;
  }

  let { viewer, onSave, onCancel, onDelete }: Props = $props();

  // 開いたときの値から編集する (開き直すたびに作り直されるので、最初の値だけでよい)
  const initial = untrack(() => viewer);
  let reading = $state(initial.reading ?? '');
  let tagsText = $state(formatTags(initial.tags));
  let notes = $state(initial.notes ?? '');
  let busy = $state(false);
  let error = $state('');
  let confirmingDelete = $state(false);

  const focusOnOpen = (node: HTMLElement) => node.focus();

  async function run(action: () => Promise<void>, failure: string) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      await action();
    } catch (e) {
      console.error(failure, e);
      error = failure;
    } finally {
      busy = false;
    }
  }

  const save = () => run(() => onSave({ reading, notes, tags: parseTags(tagsText) }), '保存に失敗しました');
  const remove = () => run(onDelete, '消すのに失敗しました');

  /** 欄で Enter → 保存 (日本語入力の変換を確定する Enter は除く) */
  function handleFieldKeydown(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing || e.keyCode === 229) return;
    e.preventDefault();
    void save();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    if (confirmingDelete) confirmingDelete = false;
    else onCancel();
  }
</script>

<!-- Esc は中の欄・ボタンから伝わってくるものをまとめて受ける (div 自体は操作しない) -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="flex flex-col gap-2 rounded-control border border-accent/40 bg-surface-low p-2 text-fg-1"
  onkeydown={handleKeydown}
>
  <div class="flex flex-wrap gap-2">
    <label class="flex min-w-40 flex-1 flex-col gap-1 text-caption text-fg-2">
      読み仮名
      <input
        bind:value={reading}
        type="text"
        placeholder="例: やまだ たろう"
        onkeydown={handleFieldKeydown}
        {@attach focusOnOpen}
        class="rounded-control border border-outline bg-surface px-2 py-1 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent"
      />
    </label>
    <label class="flex min-w-40 flex-1 flex-col gap-1 text-caption text-fg-2">
      タグ (読点・カンマで区切る)
      <input
        bind:value={tagsText}
        type="text"
        placeholder="例: 常連、攻略"
        onkeydown={handleFieldKeydown}
        class="rounded-control border border-outline bg-surface px-2 py-1 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent"
      />
    </label>
  </div>
  <label class="flex flex-col gap-1 text-caption text-fg-2">
    メモ
    <input
      bind:value={notes}
      type="text"
      placeholder="例: 攻略情報をよくくれる"
      onkeydown={handleFieldKeydown}
      class="rounded-control border border-outline bg-surface px-2 py-1 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent"
    />
  </label>

  {#if confirmingDelete}
    <div role="alert" class="flex flex-col gap-2 rounded-control border border-error bg-error/10 p-2 text-label text-fg-1">
      <p>
        取り消せません。{viewer.display_name} の記録 (読み仮名・メモ・タグ・コメント数など) をすべて消します。次にコメントしたら新しい視聴者として記録します。
      </p>
      <div class="flex gap-2">
        <button
          type="button"
          onclick={remove}
          disabled={busy}
          class="rounded-control bg-error px-3 py-1 text-label text-surface hover:opacity-90 disabled:opacity-50"
        >
          消す
        </button>
        <button
          type="button"
          onclick={() => (confirmingDelete = false)}
          {@attach focusOnOpen}
          class="rounded-control border border-outline px-3 py-1 text-label text-fg-2 hover:bg-fg-1/8"
        >
          やめる
        </button>
      </div>
    </div>
  {:else}
    <div class="flex flex-wrap items-center gap-2">
      <button
        type="button"
        onclick={save}
        disabled={busy}
        class="rounded-control bg-accent px-3 py-1 text-label text-surface hover:opacity-90 disabled:opacity-50"
      >
        保存
      </button>
      <button type="button" onclick={onCancel} class="rounded-control border border-outline px-3 py-1 text-label text-fg-2 hover:bg-fg-1/8">
        やめる
      </button>
      <span class="text-caption text-fg-2">Enter で保存</span>
      <button
        type="button"
        onclick={() => (confirmingDelete = true)}
        class="ml-auto rounded-control px-2 py-1 text-label text-error hover:bg-fg-1/8"
      >
        この視聴者を消す…
      </button>
    </div>
  {/if}
  {#if error}
    <p role="alert" class="text-caption text-error">{error}</p>
  {/if}
</div>
