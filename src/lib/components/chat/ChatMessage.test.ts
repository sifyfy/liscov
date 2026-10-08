import { describe, it, expect } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import ChatMessage from './ChatMessage.svelte';
import type { ChatMessage as ChatMessageType } from '$lib/types';

function createMessage(overrides: Partial<ChatMessageType> = {}): ChatMessageType {
	return {
		id: 'test_msg_1',
		timestamp: '2026-01-27T11:36:06+09:00',
		timestamp_usec: '0',
		author: 'TestUser',
		author_icon_url: null,
		channel_id: 'UC_test',
		content: 'テストメッセージ',
		runs: [{ type: 'Text', content: 'テストメッセージ' }],
		message_type: 'text',
		amount: null,
		is_member: false,
		is_first_time_viewer: false,
		in_stream_comment_count: null,
		metadata: null,
		// 多接続対応で追加されたフィールド
		connection_id: BigInt(1),
		platform: 'youtube',
		broadcaster_name: 'TestBroadcaster',
		author_reading: null,
		...overrides,
	};
}

// spec: docs/specs/02_chat.md「配信中の画面」(1 行の形・優先順位と強調) と「メッセージ表示仕様」
describe('ChatMessage', () => {
	const root = (container: HTMLElement) => container.querySelector('[data-message-id]') as HTMLElement;
	const renderRow = (message: ChatMessageType, props: Record<string, unknown> = {}) =>
		render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false, ...props } });

	const superchat = (overrides: Partial<ChatMessageType> = {}) =>
		createMessage({
			message_type: 'superchat',
			amount: '¥10,000',
			content: '遅れてきました！',
			runs: [{ type: 'Text', content: '遅れてきました！' }],
			metadata: {
				amount: '¥10,000',
				superchat_colors: { header_background: '#D00000', header_text: '#FFFFFF', body_background: '#E62117', body_text: '#FFFFFF' },
				badge_info: [],
				is_moderator: false,
				is_verified: false,
				milestone_months: null,
				gift_count: null,
				gift_name: null,
				gift_image_url: null,
				jewel_count: null,
				badges: [],
			},
			...overrides,
		});

	describe('1 行の形', () => {
		it('名前と本文を同じ段落に続ける', () => {
			const { container } = renderRow(
				createMessage({ author: 'しおこんぶ', content: 'このボス強すぎない？', runs: [{ type: 'Text', content: 'このボス強すぎない？' }] }),
			);
			const paragraph = root(container).querySelector('p') as HTMLElement;
			expect(paragraph.textContent).toContain('しおこんぶ');
			expect(paragraph.textContent).toContain('このボス強すぎない？');
		});

		it('長い名前・本文を省略しない (truncate・text-overflow を使わない)', () => {
			const { container } = renderRow(createMessage({ author: 'とても長い名前の視聴者さんのアカウント名です' }));
			expect(container.querySelector('.truncate')).toBeNull();
			expect(container.innerHTML).not.toContain('text-overflow');
		});

		it('読み仮名が登録されていれば名前の横に出す', () => {
			const { container } = renderRow(createMessage({ author: '@田中-abc', author_reading: 'たなか' }));
			expect(root(container).textContent).toContain('たなか');
		});

		it('パネルで登録し直した読み仮名 (reading) は、届いたときの読み仮名より優先する', () => {
			const { container } = renderRow(createMessage({ author: '@田中-abc', author_reading: 'たなか' }), { reading: 'タナカ' });
			expect(root(container).textContent).toContain('タナカ');
			expect(root(container).textContent).not.toContain('たなか');
		});

		it('パネルで読み仮名を空にしたら (reading が空文字) 出さない', () => {
			const { container } = renderRow(createMessage({ author: '@田中-abc', author_reading: 'たなか' }), { reading: '' });
			expect(root(container).textContent).not.toContain('たなか');
		});

		it('時刻は表示の設定で消せる', () => {
			const shown = renderRow(createMessage(), { showTimestamps: true });
			expect(shown.container.querySelector('[data-testid="message-time"]')).not.toBeNull();
			const hidden = renderRow(createMessage(), { showTimestamps: false });
			expect(hidden.container.querySelector('[data-testid="message-time"]')).toBeNull();
		});

		it('アイコンの画像が読めなければ頭文字にする', async () => {
			const { container } = renderRow(createMessage({ author: 'TestUser', author_icon_url: 'https://example.com/broken.png' }));
			await fireEvent.error(container.querySelector('img') as HTMLImageElement);
			expect(container.querySelector('img')).toBeNull();
			expect(root(container).textContent).toContain('T');
		});

		it('バッジの画像が読めなければ消す', async () => {
			const { container } = renderRow(
				createMessage({
					metadata: {
						...superchat().metadata!,
						superchat_colors: null,
						badge_info: [{ badge_type: 'moderator', label: 'モデレーター', tooltip: 'モデレーター', image_url: 'https://example.com/b.png' }],
					},
				}),
			);
			await fireEvent.error(container.querySelector('img[alt="モデレーター"]') as HTMLImageElement);
			expect(container.querySelector('img[alt="モデレーター"]')).toBeNull();
		});

		it('文字の大きさの設定を行に使う', () => {
			const { container } = renderRow(createMessage(), { fontSize: 18 });
			expect(root(container).style.fontSize).toBe('18px');
		});
	});

	describe('優先順位と強調', () => {
		it('新規さんの初回は優先順位 2 (塗りの印「初見」)', () => {
			const { container } = renderRow(createMessage({ is_first_time_viewer: true, in_stream_comment_count: 1 }));
			expect(root(container).dataset.priority).toBe('2');
			expect(root(container).textContent).toContain('初見');
		});

		it('新規さんの 2 回目以降は優先順位 3 (枠だけの印「初見」)', () => {
			const { container } = renderRow(createMessage({ is_first_time_viewer: true, in_stream_comment_count: 3 }));
			expect(root(container).dataset.priority).toBe('3');
			expect(root(container).textContent).toContain('初見');
		});

		it('常連の配信内初回は優先順位 4 (名前の前の点)', () => {
			const { container } = renderRow(createMessage({ in_stream_comment_count: 1 }));
			expect(root(container).dataset.priority).toBe('4');
			expect(container.querySelector('[aria-label="配信内で初めてのコメント"]')).not.toBeNull();
		});

		it('常連の 2 回目以降は優先順位 5 (印なし)', () => {
			const { container } = renderRow(createMessage({ in_stream_comment_count: 12 }));
			expect(root(container).dataset.priority).toBe('5');
			expect(root(container).textContent).not.toContain('初見');
		});

		it('配信内コメント回数 #N を出す。分からないときは出さない', () => {
			expect(root(renderRow(createMessage({ in_stream_comment_count: 5 })).container).textContent).toContain('#5');
			expect(root(renderRow(createMessage({ in_stream_comment_count: null })).container).textContent).not.toContain('#');
		});
	});

	describe('スパチャ', () => {
		it('主の配信のスパチャは優先順位 1 の面で、YouTube の body_text の色を使う', () => {
			const { container } = renderRow(superchat());
			expect(root(container).dataset.priority).toBe('1');
			const body = container.querySelector('[data-testid="paid-body"]') as HTMLElement;
			expect(body.textContent).toContain('遅れてきました！');
			// jsdom は hex を rgb にする
			expect(body.style.color).toBe('rgb(255, 255, 255)');
			expect(root(container).textContent).toContain('¥10,000');
		});

		it('主の配信のスパチャにも配信内コメント回数 #N を出す', () => {
			const { container } = renderRow(superchat({ in_stream_comment_count: 3 }));
			expect(root(container).textContent).toContain('#3');
		});

		it('行の文字色はトークン (fg-1) を使い、周りの色を引き継がない', () => {
			const { container } = renderRow(createMessage({ in_stream_comment_count: 12 }));
			expect(root(container).classList.contains('text-fg-1')).toBe(true);
		});

		// ステッカーの content は代わりの文字「[Sticker]」で、本文ではない
		it('ステッカーは本文を出さない', () => {
			const sticker = superchat({ message_type: 'supersticker', amount: '¥500', content: '[Sticker]', runs: [] });
			const primary = renderRow(sticker);
			expect(primary.container.querySelector('[data-testid="paid-body"]')).toBeNull();
			expect(root(primary.container).textContent).not.toContain('[Sticker]');
			const partner = renderRow(sticker, { origin: 'partner' });
			expect(root(partner.container).textContent).not.toContain('[Sticker]');
		});

		it('相手の配信のスパチャは優先順位 5 の行に金額の札を添える', () => {
			const { container } = renderRow(superchat({ broadcaster_name: 'あおば' }), { origin: 'partner' });
			expect(root(container).dataset.priority).toBe('5');
			expect(container.querySelector('[data-testid="paid-label"]')?.textContent).toBe('¥10,000');
		});
	});

	describe('配信元 (コラボ)', () => {
		it('相手の配信のコメントには配信者名の札を付ける', () => {
			const { container } = renderRow(createMessage({ broadcaster_name: 'あおば' }), { origin: 'partner' });
			expect(root(container).dataset.origin).toBe('partner');
			expect(container.querySelector('[data-testid="source-label"]')?.textContent).toBe('あおば');
		});

		it('主の配信のコメントには札を付けない', () => {
			const { container } = renderRow(createMessage({ broadcaster_name: '主の配信者' }));
			expect(container.querySelector('[data-testid="source-label"]')).toBeNull();
		});

		it('相手の配信の新規さんも、主の配信と同じ強さ (優先順位 2)', () => {
			const { container } = renderRow(createMessage({ is_first_time_viewer: true, in_stream_comment_count: 1 }), {
				origin: 'partner',
			});
			expect(root(container).dataset.priority).toBe('2');
		});
	});

	describe('メンバー', () => {
		it('メンバーの名前は緑の文字 (member-text)', () => {
			const { container } = renderRow(createMessage({ author: 'のりたま', is_member: true, in_stream_comment_count: 4 }));
			expect(container.querySelector('[data-testid="author-name"]')?.classList.contains('text-member-text')).toBe(true);
		});

		it('新規さんの初回の行では、メンバーでも名前は通常の色', () => {
			const { container } = renderRow(createMessage({ is_member: true, is_first_time_viewer: true, in_stream_comment_count: 1 }));
			expect(container.querySelector('[data-testid="author-name"]')?.classList.contains('text-member-text')).toBe(false);
		});

		it('バッジの画像が無いメンバーには「メンバー」の札を出す', () => {
			const { container } = renderRow(createMessage({ is_member: true, in_stream_comment_count: 4 }));
			expect(root(container).textContent).toContain('メンバー');
		});
	});

	// 02_chat.md「メッセージ表示仕様」「詰めた形」: 流れたものの欄 (新しい 2 件) とお礼の一覧で使う
	describe('詰めた形 (compact)', () => {
		it('アイコンを出さない', () => {
			const { container } = renderRow(createMessage({ author_icon_url: 'https://example.com/a.png' }), { compact: true });
			expect(container.querySelector('img')).toBeNull();
			expect(container.querySelector('[aria-hidden="true"].size-chat-icon')).toBeNull();
		});

		it('スパチャは面にせず、時刻・金額・名前・本文を 1 行に続ける (時刻は表示の設定によらず出す)', () => {
			const { container } = renderRow(superchat({ author: 'ねむいペンギン', timestamp: '21:17' }), { compact: true, showTimestamps: false });
			expect(container.querySelector('[data-testid="paid-body"]')).toBeNull();
			expect(root(container).dataset.priority).toBe('1');
			expect(root(container).textContent?.replace(/\s+/g, ' ').trim()).toBe('21:17 ¥10,000 ねむいペンギン 遅れてきました！');
		});

		it('金額の札はティアの見出しの色', () => {
			const { container } = renderRow(superchat(), { compact: true });
			const chip = container.querySelector('[data-testid="paid-chip"]') as HTMLElement;
			expect(chip.textContent).toBe('¥10,000');
			expect(chip.style.backgroundColor).toBe('rgb(208, 0, 0)');
		});

		it('メンバーシップの札は member の色で種類を出す', () => {
			const { container } = renderRow(
				createMessage({ message_type: 'membership', is_member: true, metadata: { ...superchat().metadata!, amount: null, superchat_colors: null, milestone_months: 12 } }),
				{ compact: true },
			);
			const chip = container.querySelector('[data-testid="paid-chip"]') as HTMLElement;
			expect(chip.textContent).toBe('メンバー 12 か月');
			expect(chip.classList.contains('bg-member')).toBe(true);
		});

		it('読み仮名を名前の横に出す (お礼で名前を呼ぶため)', () => {
			const { container } = renderRow(superchat({ author: '@田中-abc', author_reading: 'たなか' }), { compact: true });
			expect(root(container).textContent).toContain('(たなか)');
		});

		it('新規さんの初回は流れと同じ強調 (薄い背景 + 「初見」)', () => {
			const { container } = renderRow(createMessage({ is_first_time_viewer: true, in_stream_comment_count: 1 }), { compact: true });
			expect(root(container).dataset.priority).toBe('2');
			expect(root(container).textContent).toContain('初見');
			expect(container.querySelector('img')).toBeNull();
		});
	});

	// 02_chat.md「メッセージ表示仕様」「優先順位 1 の面」のギフト (ジュエル)
	describe('ギフト', () => {
		function createGift(jewelCount: number | null, imageUrl: string | null = 'https://www.gstatic.com/gift/press_f.png') {
			return createMessage({
				message_type: 'gift',
				author: '@viewer-a1b',
				channel_id: '',
				content: 'Press F',
				runs: [],
				metadata: {
					amount: null,
					milestone_months: null,
					gift_count: null,
					gift_name: 'Press F',
					gift_image_url: imageUrl,
					jewel_count: jewelCount,
					badges: [],
					badge_info: [],
					is_moderator: false,
					is_verified: false,
					superchat_colors: null,
				},
			});
		}

		it('画像と「{author} が {gift_name} を送りました」を表示する', () => {
			const { container } = render(ChatMessage, { props: { message: createGift(10), fontSize: 13, showTimestamps: false } });
			expect(container.textContent).toContain('@viewer-a1b が Press F を送りました');
			const image = container.querySelector('[data-testid="gift-image"]') as HTMLImageElement;
			expect(image.src).toBe('https://www.gstatic.com/gift/press_f.png');
		});

		it('jewel_count があるときだけジュエル数バッジを表示する', () => {
			const withJewels = render(ChatMessage, { props: { message: createGift(10), fontSize: 13, showTimestamps: false } });
			expect(withJewels.container.querySelector('[data-testid="gift-jewels"]')?.textContent?.trim()).toBe('10 ジュエル');

			const withoutJewels = render(ChatMessage, { props: { message: createGift(null), fontSize: 13, showTimestamps: false } });
			expect(withoutJewels.container.querySelector('[data-testid="gift-jewels"]')).toBeNull();
		});

		it('画像が無いときは文だけ表示する', () => {
			const { container } = render(ChatMessage, { props: { message: createGift(null, null), fontSize: 13, showTimestamps: false } });
			expect(container.querySelector('[data-testid="gift-image"]')).toBeNull();
			expect(container.textContent).toContain('@viewer-a1b が Press F を送りました');
		});

		it('画像が読めないときは画像を消す', async () => {
			const { container } = render(ChatMessage, { props: { message: createGift(null), fontSize: 13, showTimestamps: false } });
			const image = container.querySelector('[data-testid="gift-image"]') as HTMLImageElement;
			image.dispatchEvent(new Event('error'));
			await Promise.resolve();
			expect(container.querySelector('[data-testid="gift-image"]')).toBeNull();
		});

		it('視聴者を特定できたギフトには配信内コメント回数 #N を出す', () => {
			const message = { ...createGift(10), channel_id: 'UCgift', in_stream_comment_count: 2 };
			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			expect(root(container).textContent).toContain('#2');
		});

		it('主の配信のギフトは面で出す (優先順位 1)', () => {
			const { container } = render(ChatMessage, { props: { message: createGift(null), fontSize: 13, showTimestamps: false } });
			expect(root(container).dataset.priority).toBe('1');
		});
	});
});
