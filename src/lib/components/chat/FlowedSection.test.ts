import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import FlowedSection from './FlowedSection.svelte';
import type { ChatMessage } from '$lib/types';

// spec: docs/specs/02_chat.md「配信中の画面」の「流れたもの」
// - 欄の中の並び: 新しいものが上。新しい 2 件は全文の行、それより前は「● 金額 名前」(新規さんは「初見 名前」) の短い札
// - あるときだけ出す
// - 欄の上の境目をドラッグすると高さの上限が変わる。ダブルクリックで既定 (1/8)、↑↓ は高さの 1/50 ずつ。離したときに保存する
// - 例: 流れの部分が 800px、中身が 300px、既定の 100px から 60px 上へ引くと 0.2 (160px)
// spec: 同「配信の最後のお礼」(この配信のスパチャ等の一覧)
// - 自分の配信のスパチャ等があれば、流れたものが 0 件でも見出しの 1 行だけ出す (「流れた 0」)
// - 見出しの右端に「概要」「詳細」の切り替え。件数は概要なら「流れた N」、詳細なら「全 N 件・時刻順」
// - 詳細は時刻順の全件を全文の行で (短い札にしない)
// - 高さの上限は 1/2 まで
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
      membership_level: null,
      gift_count: null,
      gifter: null,
      gift_name: null,
      gift_image_url: null,
      jewel_count: null,
    },
  });

const renderSection = (label: string, items: readonly ChatMessage[], extra: Record<string, unknown> = {}) =>
  render(FlowedSection, {
    props: { label, items, fontSize: 13, showTimestamps: true, originOf: () => 'primary' as const, infoOf: () => undefined, ...extra },
  });

const rowIds = (container: HTMLElement) => [...container.querySelectorAll('[data-message-id]')].map((el) => el.getAttribute('data-message-id'));
const headingText = (container: HTMLElement) => container.querySelector('h3')?.textContent?.replace(/\s+/g, ' ').trim();

describe('FlowedSection', () => {
  it('中身が無ければ出さない', () => {
    const { container } = renderSection('この配信のスパチャ等の一覧', []);
    expect(container.querySelector('section')).toBeNull();
  });

  it('見出しに欄の名前と件数を出す', () => {
    const { container } = renderSection('この配信のスパチャ等の一覧', [superchat('a', '¥500')]);
    expect(container.querySelector('section')?.getAttribute('aria-label')).toBe('この配信のスパチャ等の一覧');
    expect(container.querySelector('h3')?.textContent?.replace(/\s+/g, ' ').trim()).toBe('この配信のスパチャ等の一覧 1');
  });

  it('新しい 2 件は全文の行、それより前は短い札 (● 金額 名前)', () => {
    const items = [superchat('new', '¥10,000'), superchat('mid', '¥1,000'), superchat('old', '¥200')];
    const { container } = renderSection('この配信のスパチャ等の一覧', items);
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

  describe('この配信のスパチャ等の一覧', () => {
    const label = 'この配信のスパチャ等の一覧';

    it('showEmpty なら、流れたものが 0 件でも見出しの 1 行だけ出す (流れた 0)', () => {
      const { container } = renderSection(label, [], { showEmpty: true, countLabel: '流れた 0' });
      expect(container.querySelector('section')?.getAttribute('aria-label')).toBe(label);
      expect(headingText(container)).toBe('この配信のスパチャ等の一覧 流れた 0');
      expect(rowIds(container)).toEqual([]);
    });

    it('詳細 (layout="all") は渡した順 (時刻順) に全件を全文の行で出し、短い札にしない', () => {
      const items = [superchat('a', '¥500'), superchat('b', '¥1,000'), superchat('c', '¥200'), superchat('d', '¥10,000')];
      const { container } = renderSection(label, items, { layout: 'all', countLabel: '全 4 件・時刻順' });
      expect(rowIds(container)).toEqual(['a', 'b', 'c', 'd']);
      expect(container.querySelectorAll('[data-testid="flowed-chip"]')).toHaveLength(0);
      expect(headingText(container)).toBe('この配信のスパチャ等の一覧 全 4 件・時刻順');
    });

    it('高さの上限を 1/2 まで上げられる (maxRatio)', () => {
      const { container, getByRole } = renderSection(label, [superchat('a', '¥500')], { maxRatio: 1 / 2, heightRatio: 0.45 });
      expect(getByRole('separator').getAttribute('aria-valuenow')).toBe('45');
      expect(getByRole('separator').getAttribute('aria-valuemax')).toBe('50');
      expect((container.querySelector('section') as HTMLElement).style.maxHeight).toContain('45%');
    });

    it('上限を渡さなければ 1/3 で止める', () => {
      const { getByRole } = renderSection('流れた新規さん', [message('a')], { heightRatio: 0.45 });
      expect(getByRole('separator').getAttribute('aria-valuenow')).toBe('33');
    });
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
