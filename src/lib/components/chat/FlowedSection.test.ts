import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import FlowedSection from './FlowedSection.svelte';
import type { ChatMessage } from '$lib/types';

// spec: docs/specs/02_chat.md「配信中の画面」の「流れたもの」
// - 欄の中の並び: 新しいものが上。新しい 2 件は全文の行、それより前は「● 金額 名前」(新規さんは「初見 名前」) の短い札
// - あるときだけ出す
// - 欄の上の境目をドラッグすると高さの上限が変わる。ダブルクリックで既定 (1/8)、↑↓ は高さの 1/50 ずつ。離したときに保存する
// - 例: 流れの部分が 800px、中身が 300px、既定の 100px から 60px 上へ引くと 0.2 (160px)
function message(id: string, overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    id,
    timestamp: '21:17',
    timestamp_usec: '0',
    author: `name-${id}`,
    author_icon_url: null,
    channel_id: `UC_${id}`,
    content: `本文-${id}`,
    runs: [{ type: 'Text', content: `本文-${id}` }],
    message_type: 'text',
    amount: null,
    is_member: false,
    is_first_time_viewer: true,
    in_stream_comment_count: 1,
    metadata: null,
    connection_id: 1n,
    platform: 'youtube',
    broadcaster_name: '配信者',
    author_reading: null,
    author_notes: null,
    ...overrides,
  };
}

const superchat = (id: string, amount: string) =>
  message(id, {
    message_type: 'superchat',
    amount,
    is_first_time_viewer: false,
    metadata: {
      amount,
      superchat_colors: { header_background: '#1565C0', header_text: '#FFFFFF', body_background: '#1565C0', body_text: '#FFFFFF' },
      badge_info: [],
      badges: [],
      is_moderator: false,
      is_verified: false,
      milestone_months: null,
      gift_count: null,
      gift_name: null,
      gift_image_url: null,
      jewel_count: null,
    },
  });

const renderSection = (label: string, items: readonly ChatMessage[]) =>
  render(FlowedSection, {
    props: { label, items, fontSize: 13, showTimestamps: true, originOf: () => 'primary' as const, infoOf: () => undefined },
  });

describe('FlowedSection', () => {
  it('中身が無ければ出さない', () => {
    const { container } = renderSection('流れたスパチャ等', []);
    expect(container.querySelector('section')).toBeNull();
  });

  it('見出しに欄の名前と件数を出す', () => {
    const { container } = renderSection('流れたスパチャ等', [superchat('a', '¥500')]);
    expect(container.querySelector('section')?.getAttribute('aria-label')).toBe('流れたスパチャ等');
    expect(container.querySelector('h3')?.textContent?.replace(/\s+/g, ' ').trim()).toBe('流れたスパチャ等 1');
  });

  it('新しい 2 件は全文の行、それより前は短い札 (● 金額 名前)', () => {
    const items = [superchat('new', '¥10,000'), superchat('mid', '¥1,000'), superchat('old', '¥200')];
    const { container } = renderSection('流れたスパチャ等', items);
    const rows = [...container.querySelectorAll('[data-message-id]')].map((el) => el.getAttribute('data-message-id'));
    expect(rows).toEqual(['new', 'mid']);
    const chips = [...container.querySelectorAll('[data-testid="flowed-chip"]')].map((el) => el.textContent?.replace(/\s+/g, ' ').trim());
    expect(chips).toEqual(['¥200 name-old']);
    expect(container.textContent).toContain('本文-new');
    expect(container.textContent).not.toContain('本文-old');
  });

  it('新規さんの短い札は「初見 名前」', () => {
    const items = [message('a'), message('b'), message('c')];
    const { container } = renderSection('流れた新規さん', items);
    const chips = [...container.querySelectorAll('[data-testid="flowed-chip"]')].map((el) => el.textContent?.replace(/\s+/g, ' ').trim());
    expect(chips).toEqual(['初見 name-c']);
  });

  describe('欄の高さ', () => {
    const renderResizable = (heightRatio = 0.125) => {
      const onHeightChange = vi.fn();
      const view = render(FlowedSection, {
        props: {
          label: '流れた新規さん',
          items: [message('a'), message('b')],
          fontSize: 13,
          showTimestamps: true,
          originOf: () => 'primary' as const,
          infoOf: () => undefined,
          heightRatio,
          onHeightChange,
        },
      });
      const section = view.container.querySelector('section') as HTMLElement;
      // 例: 流れの部分 800px、見えている欄 100px、中身 300px (jsdom は測れないので与える)
      Object.defineProperty(section.parentElement, 'clientHeight', { configurable: true, value: 800 });
      Object.defineProperty(section, 'offsetHeight', { configurable: true, value: 100 });
      Object.defineProperty(section, 'scrollHeight', { configurable: true, value: 300 });
      const handle = view.getByRole('separator', { name: '流れた新規さんの高さ' });
      return { ...view, section, handle, onHeightChange };
    };

    it('境目に今の割合 (%) を出し、欄の高さの上限を割合で決める', () => {
      const { section, handle } = renderResizable(0.2);
      expect(handle.getAttribute('aria-valuenow')).toBe('20');
      expect(handle.getAttribute('aria-orientation')).toBe('horizontal');
      expect(section.style.maxHeight).toContain('20%');
    });

    it('境目を 60px 上へ引いて離すと 0.2 を保存する (動かしている間も欄の高さが変わる)', async () => {
      const { section, handle, onHeightChange } = renderResizable();
      await fireEvent.pointerDown(handle, { clientY: 500, pointerId: 1 });
      await fireEvent.pointerMove(handle, { clientY: 440, pointerId: 1 });
      expect(section.style.maxHeight).toContain('20%');
      expect(onHeightChange).not.toHaveBeenCalled();
      await fireEvent.pointerUp(handle, { clientY: 440, pointerId: 1 });
      expect(onHeightChange).toHaveBeenCalledTimes(1);
      expect(onHeightChange.mock.calls[0][0]).toBeCloseTo(0.2);
    });

    it('ダブルクリックで既定 (1/8) に戻して保存する', async () => {
      const { handle, onHeightChange } = renderResizable(0.3);
      await fireEvent.dblClick(handle);
      expect(onHeightChange).toHaveBeenCalledWith(0.125);
    });

    it('↑ で高さの 1/50 (16px) 高くして保存する、↓ で低くする', async () => {
      const { handle, onHeightChange } = renderResizable();
      await fireEvent.keyDown(handle, { key: 'ArrowUp' });
      expect(onHeightChange.mock.calls[0][0]).toBeCloseTo(0.145);
      await fireEvent.keyDown(handle, { key: 'ArrowDown' });
      expect(onHeightChange.mock.calls[1][0]).toBeCloseTo(0.105);
    });
  });
});
