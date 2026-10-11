import { describe, it, expect, afterEach } from 'vitest';
import { wheelScrollsHorizontally } from './wheel-scroll';

// spec: docs/specs/02_chat.md「上の行と操作パネル」の「接続の札が上の行の幅に収まらない」
// マウスのホイールの上下でも横にスクロールする (ブラウザのタブの列と同じ)

/** 幅 300px の札の列。中身の幅を content にする (jsdom は幅を測れないので与える) */
function strip(content: number) {
	const node = document.createElement('div');
	Object.defineProperty(node, 'clientWidth', { value: 300 });
	Object.defineProperty(node, 'scrollWidth', { value: content });
	document.body.append(node);
	const cleanup = wheelScrollsHorizontally(node);
	return { node, cleanup };
}

function wheel(node: HTMLElement, init: WheelEventInit): WheelEvent {
	const event = new WheelEvent('wheel', { cancelable: true, ...init });
	node.dispatchEvent(event);
	return event;
}

describe('wheelScrollsHorizontally', () => {
	afterEach(() => document.body.replaceChildren());

	it('収まらないとき、ホイールを下に回すと右へ、上に回すと左へ動く', () => {
		const { node } = strip(900);
		const down = wheel(node, { deltaY: 100 });
		expect(node.scrollLeft).toBe(100);
		expect(down.defaultPrevented).toBe(true);
		wheel(node, { deltaY: -40 });
		expect(node.scrollLeft).toBe(60);
	});

	it('行単位のホイール (deltaMode = 1) は 1 行 16px として動く', () => {
		const { node } = strip(900);
		wheel(node, { deltaY: 3, deltaMode: WheelEvent.DOM_DELTA_LINE });
		expect(node.scrollLeft).toBe(48);
	});

	it('収まっているときは何もしない (ホイールを奪わない)', () => {
		const { node } = strip(300);
		const event = wheel(node, { deltaY: 100 });
		expect(node.scrollLeft).toBe(0);
		expect(event.defaultPrevented).toBe(false);
	});

	it('トラックパッドの左右・Shift + ホイールは今までどおり (ブラウザに任せる)', () => {
		const { node } = strip(900);
		expect(wheel(node, { deltaX: 50, deltaY: 2 }).defaultPrevented).toBe(false);
		expect(wheel(node, { deltaY: 100, shiftKey: true }).defaultPrevented).toBe(false);
		expect(node.scrollLeft).toBe(0);
	});

	it('外すとホイールに反応しない', () => {
		const { node, cleanup } = strip(900);
		cleanup();
		expect(wheel(node, { deltaY: 100 }).defaultPrevented).toBe(false);
	});
});
