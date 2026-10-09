<script lang="ts">
  /**
   * 視聴者のタブの一覧 (docs/specs/06_viewer.md「視聴者のタブ (配信の後・準備)」)
   *
   * 2 行の一覧 (試作 src/routes/design-proto/viewers の C)。
   * 1 行目に名前 (読み仮名)・タグ、2 行目に最終確認の日付・コメント数・貢献額 (0 なら出さない)・メモ。長い名前・メモは折り返す。
   * 行を押すとその行のすぐ下に編集 (ViewerEditor) を広げる。開いている間に別の行を押すと、前の編集は捨てて押した行に開く
   */
  import type { ViewerWithCustomInfo } from '$lib/types';
  import { formatContribution, formatCount, formatDay } from '$lib/utils/format';
  import ViewerEditor from './ViewerEditor.svelte';

  interface Props {
    viewers: readonly ViewerWithCustomInfo[];
    /** 保存する。失敗したら reject (編集は開いたまま) */
    onSave: (viewer: ViewerWithCustomInfo, values: { reading: string; notes: string; tags: string[] }) => Promise<void>;
    /** 消す。失敗したら reject (編集は開いたまま) */
    onDelete: (viewer: ViewerWithCustomInfo) => Promise<void>;
  }

  let { viewers, onSave, onDelete }: Props = $props();

  // 編集を開いている視聴者 (プロフィール id)
  let openId = $state<number | null>(null);
  // 閉じたらフォーカスを行に戻す (キーボードで続けて整理できるように)
  const rowButtons: Record<number, HTMLButtonElement> = {};

  function close(id: number) {
    openId = null;
    rowButtons[id]?.focus({ preventScroll: true });
  }
</script>

<ul class="min-h-0 flex-1 overflow-y-auto">
  {#each viewers as v (v.id)}
    {@const open = openId === v.id}
    <li class="border-b border-outline-variant {open ? 'bg-accent/10' : ''}">
      <button
        type="button"
        aria-expanded={open}
        bind:this={rowButtons[v.id]}
        onclick={() => (openId = v.id)}
        class="flex w-full flex-col gap-0.5 px-3 py-1.5 text-left text-label hover:bg-fg-1/4"
      >
        <span class="flex flex-wrap items-center gap-x-1 gap-y-0.5">
          <span class="font-bold break-words text-fg-1">{v.display_name}</span>
          {#if v.reading}<span class="text-caption text-fg-2">({v.reading})</span>{/if}
          {#each v.tags as tag (tag)}
            <span class="rounded-control border border-outline px-1.5 text-caption whitespace-nowrap text-fg-2">{tag}</span>
          {/each}
        </span>
        <span class="flex flex-wrap gap-x-3 text-caption text-fg-2">
          <span class="tabular-nums">最終 {formatDay(v.last_seen)}</span>
          <span class="tabular-nums">コメント {formatCount(v.message_count)}</span>
          {#if v.total_contribution > 0}<span class="tabular-nums">{formatContribution(v.total_contribution)}</span>{/if}
          {#if v.notes}<span class="min-w-0 break-words text-fg-1">{v.notes}</span>{/if}
        </span>
      </button>
      {#if open}
        <div class="mx-3 mb-2">
          <ViewerEditor
            viewer={v}
            onSave={async (values) => {
              await onSave(v, values);
              close(v.id);
            }}
            onCancel={() => close(v.id)}
            onDelete={async () => {
              await onDelete(v);
              openId = null;
            }}
          />
        </div>
      {/if}
    </li>
  {/each}
</ul>
