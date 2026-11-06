import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { collect, inThread, orphanRoot, parseThreadRoute, threadRoute } from './thread.ts';

const line = (id: string, parent?: string, thread?: string, extra: Partial<ChatMessage> = {}) =>
  ({
    id,
    ts: 0,
    body: [],
    flags: [],
    ...(parent ? { reply: { parentId: parent, parentLogin: 'op', parentDisplay: 'Op', parentBody: 'first', threadId: thread ?? parent } } : {}),
    ...extra,
  }) as ChatMessage;

test('V109-C01 the thread is its first line and every reply carrying its id, in order', () => {
  const list = [line('x'), line('t'), line('a', 't'), line('y', 'x'), line('b', 'a', 't'), line('t')];
  assert.deepEqual(collect(list, 't').map((m) => m.id), ['t', 'a', 'b'], 'repeated id dropped');
  assert.deepEqual(collect(list, 'x').map((m) => m.id), ['x', 'y']);
  assert.deepEqual(collect(list, 'nope'), []);
});

test('V109-C01 notices never belong to a thread', () => {
  assert.equal(inThread(line('t', undefined, undefined, { notice: {} as never }), 't'), false);
  assert.equal(inThread(line('a', 't', undefined, { notice: {} as never }), 't'), false);
});

test('V109-C01 a first line that left the buffer shows what the reply tag carries', () => {
  const list = [line('a', 't'), line('b', 'a', 't')];
  assert.deepEqual(orphanRoot(list, 't'), { login: 'op', display: 'Op', body: 'first' });
  assert.equal(orphanRoot([line('t'), ...list], 't'), null, 'still here');
  assert.equal(orphanRoot([line('b', 'a', 't')], 't'), null, 'no direct reply: nothing known');
});

test('V109-C01 the route round-trips', () => {
  assert.equal(threadRoute('xqc', 'm1'), 'thread/xqc/m1');
  assert.deepEqual(parseThreadRoute('thread/xqc/m1'), { channel: 'xqc', threadId: 'm1' });
  assert.deepEqual(parseThreadRoute('thread/xqc'), { channel: 'xqc', threadId: '' });
});
