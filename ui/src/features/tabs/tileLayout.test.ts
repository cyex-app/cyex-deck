import assert from 'node:assert/strict';
import { test } from 'node:test';

import { leaf, leaves, move, split, type Split } from '../../lib/panels.ts';
import { chatSideOf, shareOf, tileTree, turnedShares } from './tileLayout.ts';

const parts = { stream: true, chat: true, modview: false, side: 'right' as const, ratio: 0.28, modRow: true, modShare: 0.5 };

test('the default layout: the mod view splits the stream area, the chat on its side', () => {
  assert.deepEqual(tileTree(parts), split('row', [leaf('stream'), leaf('chat')], [0.72, 0.28]));
  const all = tileTree({ ...parts, modview: true, side: 'top' }) as Split;
  assert.equal(all.dir, 'column');
  assert.deepEqual(leaves(all), ['chat', 'modview', 'stream']);
  assert.deepEqual(leaves(tileTree({ ...parts, modview: true, modRow: false })), ['stream', 'modview', 'chat']);
  // V103-07 The stream shown in the mod view: the mod view alone beside the chat.
  assert.deepEqual(leaves(tileTree({ ...parts, stream: false, modview: true })), ['modview', 'chat']);
  assert.equal(tileTree({ ...parts, stream: false, chat: false }), null);
});

test('dragging the chat or the stream keeps V102-23: a side for the chat', () => {
  const t = tileTree(parts);
  // The chat dropped under the stream goes under it; the stream dropped under the chat
  // leaves the chat on top.
  assert.equal(chatSideOf(move(t, 'chat', 'stream', 'bottom')), 'bottom');
  assert.equal(chatSideOf(move(t, 'stream', 'chat', 'bottom')), 'top');
  assert.equal(chatSideOf(move(t, 'stream', 'chat', 'left')), 'right');
  // V103-05 With the mod view in it, it's an arrangement of its own.
  const three = tileTree({ ...parts, modview: true });
  assert.equal(chatSideOf(move(three, 'modview', 'chat', 'bottom')), null);
});

test('dividers move the chat share or the mod view share', () => {
  assert.deepEqual(shareOf(tileTree(parts) as Split), { kind: 'chat', index: 1 });
  const inner = (tileTree({ ...parts, modview: true }) as Split).parts[0] as Split;
  assert.deepEqual(shareOf(inner), { kind: 'modview', index: 0 });
  assert.equal(shareOf(split('row', [leaf('a'), leaf('b')]) as Split), null);
});

test('turned round, the stream keeps its 16:9 height above the chat', () => {
  const row = tileTree(parts) as Split;
  const [stream, chat] = turnedShares(row, { w: 420, h: 700 })!;
  assert.equal(Math.round(chat * 700), 700 - Math.round((420 * 9) / 16));
  assert.ok(Math.abs(stream + chat - 1) < 1e-9);
  assert.equal(turnedShares(split('row', [leaf('a'), leaf('b')]) as Split, { w: 1, h: 1 }), null);
});
