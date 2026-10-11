<script lang="ts">
  /**
   * 参加型の一覧 (参加中・待機)。ブリーフ §1「コラボの参加型の管理」
   * - layout: row は 2 行に詰める (狭い幅の帯)、column は縦に並べる (パネル)
   */
  import { party, partnerName } from './data';

  let { layout = 'row' }: { layout?: 'row' | 'column' } = $props();

  const GROUPS = [
    { label: '参加中', players: party.playing },
    { label: '待機', players: party.waiting },
  ] as const;
</script>

{#snippet player(name: string, partner: boolean)}
  <span class="whitespace-nowrap">
    {#if partner}<span class="mr-0.5 rounded-control bg-surface-highest px-1 text-chat-meta text-fg-2">{partnerName}</span>{/if}{name}
  </span>
{/snippet}

{#if layout === 'row'}
  <!-- 番号つきで、省略せず折り返す -->
  <div class="grid gap-0.5 px-3 py-1.5 text-chat-body">
    {#each GROUPS as g (g.label)}
      <div class="flex items-baseline gap-2">
        <span class="shrink-0 text-label whitespace-nowrap text-fg-2">{g.label} <span class="tabular-nums">{g.players.length}</span></span>
        <ol class="flex min-w-0 flex-wrap gap-x-3">
          {#each g.players as p, i (p.name)}
            <li class="whitespace-nowrap"><span class="mr-1 text-chat-meta text-fg-3 tabular-nums">{i + 1}</span>{@render player(p.name, p.origin === 'partner')}</li>
          {/each}
        </ol>
      </div>
    {/each}
  </div>
{:else}
  <div class="grid gap-3 px-3 py-2 text-chat-body">
    {#each GROUPS as g (g.label)}
      <section>
        <h3 class="text-label text-fg-2">{g.label} <span class="tabular-nums">{g.players.length}</span></h3>
        <ol class="mt-1 grid gap-1">
          {#each g.players as p, i (p.name)}
            <li class="flex items-baseline gap-2"><span class="w-4 text-chat-meta text-fg-3 tabular-nums">{i + 1}</span>{@render player(p.name, p.origin === 'partner')}</li>
          {/each}
        </ol>
      </section>
    {/each}
  </div>
{/if}
