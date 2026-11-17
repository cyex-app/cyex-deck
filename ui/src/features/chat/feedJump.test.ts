import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Tab } from '../../ipc/bindings/Tab';
import { chatSpot, inChatCheck, newestFirst, ordered, type Here } from './feedJump.ts';

test('V111-53 mention tabs read newest first, custom feeds keep chat order', () => {
  assert.ok(newestFirst('Mentions'));
  assert.ok(newestFirst(' global mentions '));
  assert.ok(!newestFirst('Pings'));
  const list = [1, 2, 3];
  assert.deepEqual(ordered(list, true), [3, 2, 1]);
  assert.equal(ordered(list, false), list);
  assert.deepEqual(list, [1, 2, 3], 'the kept list is not reversed in place');
});

const tile = (id: string, login: string, more: Record<string, unknown> = {}) => ({ id, channel: { id, login, name: login }, feed: null, show: { player: true, chat: true }, ...more });
const tab = (id: string, ...tiles: ReturnType<typeof tile>[]) => ({ id, columns: [{ tiles }] }) as unknown as Tab;
const main: Here = { active: null, popped: new Set(), poppedChats: new Set(), only: null };

test('V111-53 the chat to show a line in: the tab on screen first, never a popped-out one', () => {
  const list = [tab('a', tile('a1', 'xqc')), tab('b', tile('b1', 'xqc')), tab('c', tile('c1', 'forsen'))];
  assert.deepEqual(chatSpot(list, 'XQC', main), { tab: 'a', tile: 'a1' });
  assert.deepEqual(chatSpot(list, 'xqc', { ...main, active: 'b' }), { tab: 'b', tile: 'b1' });
  assert.deepEqual(chatSpot(list, 'xqc', { ...main, popped: new Set(['a']) }), { tab: 'b', tile: 'b1' });
  assert.deepEqual(chatSpot(list, 'xqc', { ...main, poppedChats: new Set(['a1', 'b1']) }), null);
  assert.equal(chatSpot(list, 'nobody', main), null);
  // A tab's own window only looks in that tab; a window with no tab finds none.
  assert.deepEqual(chatSpot(list, 'xqc', { ...main, only: 'b' }), { tab: 'b', tile: 'b1' });
  assert.equal(chatSpot(list, 'xqc', { ...main, only: '' }), null);
});

test('V111-53 stream-only tiles and feed tiles are no chat', () => {
  const list = [tab('a', tile('a1', 'xqc', { show: { player: true, chat: false } }), tile('a2', 'xqc', { feed: 'Mentions' }))];
  assert.equal(chatSpot(list, 'xqc', main), null);
});

test('V111-53 Show in chat only while the room still has the line, one check per line for a while', async () => {
  let now = 0;
  const asked: string[] = [];
  const have = new Set(['xqc/m1']);
  const check = inChatCheck(
    (c) => (c === 'xqc' ? { tab: 'a', tile: 'a1' } : null),
    async (c, id) => (asked.push(`${c}/${id}`), have.has(`${c}/${id}`)),
    () => now,
  );
  assert.deepEqual(await check('xqc', 'm1'), { tab: 'a', tile: 'a1' });
  assert.equal(await check('xqc', 'm2'), null);
  assert.equal(await check('forsen', 'm1'), null, 'no chat here: no call');
  assert.equal(await check('xqc', 'local-1'), null, 'local lines: no call');
  await check('xqc', 'm1');
  assert.deepEqual(asked, ['xqc/m1', 'xqc/m2']);
  // The line scrolled out of the room: a later check says so.
  have.clear();
  now = 5000;
  assert.equal(await check('xqc', 'm1'), null);
  assert.equal(asked.length, 3);
});

test('V111-53 a failed check offers no Show in chat', async () => {
  const check = inChatCheck(
    () => ({ tab: 'a', tile: 'a1' }),
    () => Promise.reject(new Error('gone')),
  );
  assert.equal(await check('xqc', 'm1'), null);
});
