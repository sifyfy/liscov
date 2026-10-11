// 横にスクロールする列を、マウスのホイールの上下でも横に動かす ({@attach})
// 02_chat.md「上の行と操作パネル」の「接続の札が上の行の幅に収まらない」

/** 行単位のホイール (deltaMode = 1) の 1 行の大きさ */
const LINE_PX = 16;

export function wheelScrollsHorizontally(node: HTMLElement): () => void {
	const onWheel = (event: WheelEvent) => {
		// 収まっているとき・横の操作 (トラックパッドの左右、Shift + ホイール) はブラウザに任せる
		if (node.scrollWidth <= node.clientWidth) return;
		if (event.deltaX !== 0 || event.shiftKey || event.deltaY === 0) return;
		event.preventDefault();
		node.scrollLeft += event.deltaMode === WheelEvent.DOM_DELTA_LINE ? event.deltaY * LINE_PX : event.deltaY;
	};
	// preventDefault するので passive にしない
	node.addEventListener('wheel', onWheel, { passive: false });
	return () => node.removeEventListener('wheel', onWheel);
}
