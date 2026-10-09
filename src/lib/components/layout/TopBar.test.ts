import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import type { ComponentProps } from 'svelte';
import TopBar, { type AppTab } from './TopBar.svelte';
import { emptyReactions } from '$lib/utils/reactions';
import type { FrontendConnectionState } from '$lib/types';

// spec: docs/specs/02_chat.md「配信中の画面」の「上の行と操作パネル」
function conn(id: number, name: string, state: FrontendConnectionState['connectionState'] = 'connected'): FrontendConnectionState {
  return {
    id,
    platform: 'youtube',
    streamUrl: `https://www.youtube.com/watch?v=${id}`,
    streamTitle: `${name} の配信`,
    broadcasterName: name,
    broadcasterChannelId: `UC${id}`,
    connectionState: state,
    color: '#888888',
    jewelCountUnavailable: false,
    reactions: emptyReactions,
  };
}

function setup(overrides: Partial<ComponentProps<typeof TopBar>> = {}) {
  const callbacks = {
    onSelectTab: vi.fn<(tab: AppTab) => void>(),
    onTogglePanel: vi.fn<() => void>(),
    onAddConnection: vi.fn<() => void>(),
    onDisconnect: vi.fn<(connectionId: number) => void>(),
  };
  render(TopBar, {
    props: {
      activeTab: 'chat',
      connections: [conn(1, '配信者A'), conn(2, 'あおば', 'connecting')],
      primaryId: 1,
      panelOpen: false,
      ...callbacks,
      ...overrides,
    },
  });
  return callbacks;
}

describe('TopBar', () => {
  it('画面のタブはアイコン。名前はヒントと読み上げ用の名前で出し、押すと切り替える', async () => {
    const props = setup();
    const viewers = screen.getByRole('button', { name: '視聴者' });
    expect(viewers.getAttribute('title')).toBe('視聴者');
    expect(screen.getByRole('button', { name: 'チャット' }).getAttribute('aria-current')).toBe('page');
    await fireEvent.click(viewers);
    expect(props.onSelectTab).toHaveBeenCalledWith('viewers');
  });

  it('接続の札: 接続済みは success の点、接続の途中は控えめな点。主の配信は太字。配信タイトルはヒント', () => {
    setup();
    const [a, b] = screen.getAllByTestId('connection-chip');
    expect(a.querySelector('[data-testid="chip-dot"]')?.className).toContain('bg-success');
    expect(b.querySelector('[data-testid="chip-dot"]')?.className).toContain('bg-fg-3');
    expect(a.querySelector('.font-bold')?.textContent).toBe('配信者A');
    expect(b.querySelector('.font-bold')).toBeNull();
    expect(a.getAttribute('title')).toBe('配信者A の配信');
  });

  it('札の × で切断する。接続の途中の接続は切断できない', async () => {
    const props = setup();
    await fireEvent.click(screen.getByRole('button', { name: '配信者A を切断' }));
    expect(props.onDisconnect).toHaveBeenCalledWith(1);
    expect((screen.getByRole('button', { name: 'あおば を切断' }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('「+」で配信に接続 (操作パネルを開いて URL の入力へ)', async () => {
    const props = setup();
    await fireEvent.click(screen.getByRole('button', { name: '配信に接続' }));
    expect(props.onAddConnection).toHaveBeenCalledOnce();
  });

  it('右端の矢印で操作パネルを開け閉めする', async () => {
    const props = setup();
    const toggle = screen.getByRole('button', { name: '操作パネルを開く' });
    expect(toggle.getAttribute('aria-expanded')).toBe('false');
    await fireEvent.click(toggle);
    expect(props.onTogglePanel).toHaveBeenCalledOnce();
  });

  it('操作パネルが開いているときは「操作パネルを閉じる」', () => {
    setup({ panelOpen: true });
    expect(screen.getByRole('button', { name: '操作パネルを閉じる' }).getAttribute('aria-expanded')).toBe('true');
  });
});
