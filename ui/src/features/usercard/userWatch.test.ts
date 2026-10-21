import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { RoomEvent } from '../../ipc/bindings/RoomEvent';
import { KEEP, theirIds, touches } from './userWatch.ts';

const line = (id: string, user: string, notice = false) => ({ id, sender: { id: user }, notice: notice ? {} : null }) as unknown as ChatMessage;

test('V107-H1 only their line ids are kept, capped', () => {
  assert.deepEqual(theirIds([line('1', 'u'), line('2', 'v'), line('3', 'u', true)], 'u'), ['1']);
  const many = Array.from({ length: KEEP + 5 }, (_, i) => line(String(i), 'u'));
  const ids = theirIds(many, 'u');
  assert.equal(ids.length, KEEP);
  assert.equal(ids[0], '5');
  touches([{ type: 'message', message: line('x', 'u') }], 'u', ids);
  assert.equal(ids.length, KEEP);
  assert.equal(ids.at(-1), 'x');
});

test('V107-R23 a new, edited or deleted line of theirs counts; others don\'t', () => {
  const ids = ['1'];
  const ev = (e: RoomEvent) => touches([e], 'u', ids);
  assert.equal(ev({ type: 'message', message: line('9', 'v') }), false);
  assert.equal(ev({ type: 'message', message: line('2', 'u') }), true);
  assert.equal(ev({ type: 'message', message: line('2', 'u') }), false);
  assert.equal(ev({ type: 'update', message: line('1', 'u') }), true);
  assert.equal(ev({ type: 'delete', id: '9' }), false);
  assert.equal(ev({ type: 'delete', id: '1' }), true);
  assert.equal(ev({ type: 'clearUser', userId: 'u', gone: {} as never }), true);
  assert.equal(ev({ type: 'clearUser', userId: 'v', gone: {} as never }), false);
  assert.equal(ev({ type: 'history', messages: [line('3', 'u')] }), true);
  assert.equal(ev({ type: 'wipe' }), true);
  assert.equal(ev({ type: 'status', status: 'connected' as never }), false);
});
