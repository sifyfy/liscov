import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import FlowedSection from './FlowedSection.svelte';
import type { ChatMessage } from '$lib/types';

// spec: docs/specs/02_chat.md「配信中の画面」の「流れたもの」
// - 欄の中の並び: 新しいものが上。新しい 2 件は全文の行、それより前は「● 金額 名前」(新規さんは「初見 名前」) の短い札
// - あるときだけ出す
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
});
