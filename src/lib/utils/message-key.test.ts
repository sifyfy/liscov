import { describe, it, expect } from 'vitest';
import { messageKey } from './message-key';

// spec: 02_chat.md 制約「メッセージ重複排除キーは connection_id:message_id の複合キー」
describe('messageKey', () => {
  it('同じ message_id でも接続が違えば別のキーになる', () => {
    expect(messageKey({ connection_id: BigInt(1), id: 'm1' })).not.toBe(
      messageKey({ connection_id: BigInt(2), id: 'm1' })
    );
  });

  it('同じ接続の同じ message_id は同じキーになる', () => {
    expect(messageKey({ connection_id: BigInt(1), id: 'm1' })).toBe(
      messageKey({ connection_id: BigInt(1), id: 'm1' })
    );
  });
});
