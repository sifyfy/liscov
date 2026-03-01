import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
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
		...overrides,
	};
}

describe('ChatMessage', () => {
	describe('スーパーチャットのテキスト色', () => {
		it('スーパーチャットはYouTubeから取得したbody_text色を適用する', () => {
			const message = createMessage({
				message_type: 'superchat',
				amount: '¥3,000',
				metadata: {
					amount: '¥3,000',
						superchat_colors: {
						header_background: '#E62117',
						header_text: '#FFFFFF',
						body_background: '#FFB300',
						body_text: '#000000',
					},
					badge_info: [],
					is_moderator: false,
					is_verified: false,
					milestone_months: null,
					gift_count: null,
					badges: [],
				},
			});

			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			const textElement = container.querySelector('.mt-1.ml-8 p') as HTMLElement;

			// jsdom converts hex to rgb
			expect(textElement.style.color).toBe('rgb(0, 0, 0)');
		});

		it('スーパーステッカーはYouTubeから取得したbody_text色を適用する', () => {
			const message = createMessage({
				message_type: 'supersticker',
				amount: '¥500',
				metadata: {
					amount: '¥500',
						superchat_colors: {
						header_background: '#00BCD4',
						header_text: '#FFFFFF',
						body_background: '#00BCD4',
						body_text: '#FFFFFF',
					},
					badge_info: [],
					is_moderator: false,
					is_verified: false,
					milestone_months: null,
					gift_count: null,
					badges: [],
				},
			});

			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			const textElement = container.querySelector('.mt-1.ml-8 p') as HTMLElement;

			// jsdom converts hex to rgb
			expect(textElement.style.color).toBe('rgb(255, 255, 255)');
		});

		it('通常メッセージはデフォルトのCSS変数テキスト色を使用する', () => {
			const message = createMessage({ message_type: 'text' });

			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			const textElement = container.querySelector('.mt-1.ml-8 p') as HTMLElement;

			expect(textElement.style.color).toBe('var(--text-secondary)');
		});

		it('superchat_colorsがある場合はメタデータ行に白背景を適用する', () => {
			const message = createMessage({
				message_type: 'superchat',
				amount: '¥3,000',
				metadata: {
					amount: '¥3,000',
						superchat_colors: {
						header_background: '#E62117',
						header_text: '#FFFFFF',
						body_background: '#FFB300',
						body_text: '#000000',
					},
					badge_info: [],
					is_moderator: false,
					is_verified: false,
					milestone_months: null,
					gift_count: null,
					badges: [],
				},
			});

			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			const metadataRow = container.querySelector('.flex.items-center.gap-2') as HTMLElement;

			expect(metadataRow.classList.contains('bg-[var(--bg-surface-2)]/80')).toBe(true);
		});

		it('通常メッセージのメタデータ行には白背景を適用しない', () => {
			const message = createMessage({ message_type: 'text' });

			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			const metadataRow = container.querySelector('.flex.items-center.gap-2') as HTMLElement;

			expect(metadataRow.classList.contains('bg-[var(--bg-surface-2)]/80')).toBe(false);
		});

		it('superchat_colorsがnullの場合はデフォルト色を使用する', () => {
			const message = createMessage({
				message_type: 'superchat',
				amount: '¥100',
				metadata: {
					amount: '¥100',
						superchat_colors: null,
					badge_info: [],
					is_moderator: false,
					is_verified: false,
					milestone_months: null,
					gift_count: null,
					badges: [],
				},
			});

			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			const textElement = container.querySelector('.mt-1.ml-8 p') as HTMLElement;

			expect(textElement.style.color).toBe('var(--text-secondary)');
		});
	});

	describe('初見さんバッジ', () => {
		it('is_first_time_viewer=true + count=1 のとき 🎉初見さん バッジを表示する', () => {
			const message = createMessage({ is_first_time_viewer: true, in_stream_comment_count: 1 });
			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			expect(container.textContent).toContain('🎉初見さん');
		});

		it('is_first_time_viewer=true + count>1 のとき muted な 初見さん を表示する', () => {
			const message = createMessage({ is_first_time_viewer: true, in_stream_comment_count: 3 });
			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			expect(container.textContent).toContain('初見さん');
			expect(container.textContent).not.toContain('🎉初見さん');
		});

		it('is_first_time_viewer=false のとき 初見さん を表示しない', () => {
			const message = createMessage({ is_first_time_viewer: false });
			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			expect(container.textContent).not.toContain('初見さん');
		});
	});

	describe('配信内コメント回数', () => {
		it('in_stream_comment_count=1 のとき #1 を表示する', () => {
			const message = createMessage({ in_stream_comment_count: 1 });
			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			expect(container.textContent).toContain('#1');
		});

		it('in_stream_comment_count=5 のとき #5 を表示する', () => {
			const message = createMessage({ in_stream_comment_count: 5 });
			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			expect(container.textContent).toContain('#5');
		});

		it('in_stream_comment_count=null のとき回数を表示しない', () => {
			const message = createMessage({ in_stream_comment_count: null });
			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			// '#' 記号がないことを確認（他の要素に含まれないよう確認）
			const metadataRow = container.querySelector('.flex.items-center.gap-2');
			expect(metadataRow?.textContent).not.toMatch(/#\d/);
		});
	});

	describe('初見さんバッジと配信内コメント回数の同時表示', () => {
		it('is_first_time_viewer=true + in_stream_comment_count=1 のとき 🎉初見さん と #1 を表示する', () => {
			const message = createMessage({ is_first_time_viewer: true, in_stream_comment_count: 1 });
			const { container } = render(ChatMessage, { props: { message, fontSize: 13, showTimestamps: false } });
			expect(container.textContent).toContain('🎉初見さん');
			expect(container.textContent).toContain('#1');
		});
	});
});
