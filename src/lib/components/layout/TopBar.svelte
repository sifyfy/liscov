<script module lang="ts">
  export type AppTab = 'chat' | 'viewers' | 'analytics' | 'settings';
</script>

<script lang="ts">
  /**
   * 上の行 (docs/specs/02_chat.md「配信中の画面」の「上の行と操作パネル」)
   *
   * 画面のタブ (アイコン)・接続の札・配信に接続する「+」・操作パネルを開け閉めする矢印 (右端)。
   * 配信中もずっと出ているので、控えめに 1 行に収める。値は AppShell が渡す
   */
  import Icon from '$lib/components/ui/Icon.svelte';
  import type { IconName } from '$lib/components/ui/Icon.svelte';
  import type { FrontendConnectionState } from '$lib/types';

  interface Props {
    activeTab: AppTab;
    onSelectTab: (tab: AppTab) => void;
    /** 接続中の配信 (接続した順) */
    connections: readonly FrontendConnectionState[];
    /** 主の配信の接続の id */
    primaryId: number | undefined;
    panelOpen: boolean;
    onTogglePanel: () => void;
    onAddConnection: () => void;
    onDisconnect: (connectionId: number) => void;
  }

  let { activeTab, onSelectTab, connections, primaryId, panelOpen, onTogglePanel, onAddConnection, onDisconnect }: Props =
    $props();

  const TABS = [
    { id: 'chat', icon: 'chat', label: 'チャット' },
    { id: 'viewers', icon: 'users', label: '視聴者' },
    { id: 'analytics', icon: 'chart', label: '分析' },
    { id: 'settings', icon: 'settings', label: '設定' },
  ] as const satisfies readonly { id: AppTab; icon: IconName; label: string }[];
</script>

<header class="flex h-10 shrink-0 items-center gap-2 border-b border-outline-variant bg-surface-low px-2">
  <nav class="flex shrink-0 items-center gap-0.5" aria-label="画面">
    {#each TABS as tab (tab.id)}
      <button
        type="button"
        title={tab.label}
        aria-label={tab.label}
        aria-current={activeTab === tab.id ? 'page' : undefined}
        onclick={() => onSelectTab(tab.id)}
        class="flex size-8 items-center justify-center rounded-control {activeTab === tab.id
          ? 'bg-accent/15 text-accent'
          : 'text-fg-2 hover:bg-fg-1/8'}"
      >
        <Icon name={tab.icon} size={16} />
      </button>
    {/each}
  </nav>
  <span class="h-5 w-px shrink-0 bg-outline-variant" aria-hidden="true"></span>

  <!-- 接続の札。多いときは横にスクロールする (1 行に保つ) -->
  <div class="flex min-w-0 flex-1 items-center gap-1.5 overflow-x-auto">
    {#each connections as conn (conn.id)}
      <span
        data-testid="connection-chip"
        title={conn.streamTitle}
        class="flex shrink-0 items-center gap-1 rounded-control bg-surface-high py-0.5 pr-0.5 pl-2 text-caption whitespace-nowrap text-fg-1"
      >
        <span
          data-testid="chip-dot"
          class="size-1.5 rounded-full {conn.connectionState === 'connected' ? 'bg-success' : 'bg-fg-3'}"
          aria-hidden="true"
        ></span>
        <span class={conn.id === primaryId ? 'font-bold' : ''}>{conn.broadcasterName}</span>
        <button
          type="button"
          aria-label="{conn.broadcasterName} を切断"
          title="切断"
          disabled={conn.connectionState !== 'connected'}
          onclick={() => onDisconnect(conn.id)}
          class="flex size-5 items-center justify-center rounded-control text-fg-2 hover:bg-fg-1/8 disabled:opacity-50"
        >
          <Icon name="close" size={12} />
        </button>
      </span>
    {/each}
    <button
      type="button"
      aria-label="配信に接続"
      title="配信に接続"
      onclick={onAddConnection}
      class="flex size-7 shrink-0 items-center justify-center rounded-control text-fg-2 hover:bg-fg-1/8"
    >
      <Icon name="plus" size={16} />
    </button>
  </div>

  <button
    type="button"
    aria-label={panelOpen ? '操作パネルを閉じる' : '操作パネルを開く'}
    title={panelOpen ? '操作パネルを閉じる' : '操作パネルを開く'}
    aria-expanded={panelOpen}
    onclick={onTogglePanel}
    class="flex size-8 shrink-0 items-center justify-center rounded-control {panelOpen
      ? 'bg-accent/15 text-accent'
      : 'text-fg-2 hover:bg-fg-1/8'}"
  >
    <Icon name="chevron-down" size={16} class={panelOpen ? 'rotate-180' : ''} />
  </button>
</header>
