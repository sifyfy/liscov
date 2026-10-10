import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import ThanksList from './ThanksList.svelte';
import type { ChatMessage } from '$lib/types';

// spec: docs/specs/02_chat.md「配信中の画面」の「配信の最後のお礼」
// - 一覧の中身: 自分の配信の優先順位 1 を時刻順 (古いものが上)
// - 「閉じる」を押すと閉じる
function superchat(id: string, timestamp: string): ChatMessage {
  return {
    id,
    timestamp,
    timestamp_usec: '0',
    author: `name-${id}`,
    author_icon_url: null,
    channel_id: `UC_${id}`,
    content: '',
    runs: [],
    message_type: 'superchat',
    amount: '¥500',
    is_member: false,
    is_first_time_viewer: false,
    in_stream_comment_count: 1,
    metadata: null,
    connection_id: 1n,
    platform: 'youtube',
    broadcaster_name: '配信者',
    author_reading: null,
    author_notes: null,
  };
}

const renderList = (items: readonly ChatMessage[], onClose = vi.fn()) =>
  render(ThanksList, {
    props: { items, fontSize: 13, showTimestamps: true, originOf: () => 'primary' as const, infoOf: () => undefined, onClose },
  });

describe('ThanksList', () => {
  it('渡された順 (時刻順) にすべて並べ、件数を出す', () => {
    const { container } = renderList([superchat('a', '21:10'), superchat('b', '21:20'), superchat('c', '21:30')]);
    const rows = [...container.querySelectorAll('[data-message-id]')].map((el) => el.getAttribute('data-message-id'));
    expect(rows).toEqual(['a', 'b', 'c']);
    expect(container.querySelector('h2')?.textContent).toContain('3 件');
  });

  it('「閉じる」で onClose を呼ぶ', async () => {
    const onClose = vi.fn();
    const { getByRole } = renderList([superchat('a', '21:10')], onClose);
    await fireEvent.click(getByRole('button', { name: '閉じる' }));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
