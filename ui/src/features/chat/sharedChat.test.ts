import assert from 'node:assert/strict';
import { test } from 'node:test';

import { SHARED_SEEN_MS, hiddenCount, inSharedSession, onlyHere } from './sharedChat.ts';

const src = { roomId: '2', badges: [] };
const line = (ts: number, source?: typeof src) => ({ ts, source });

test('V108-N12 a recent foreign line means a shared session', () => {
  const now = 1_000_000;
  assert.equal(inSharedSession([line(now - 5000, src), line(now - 1000)], now), true);
});

test('V108-N12 the session lets go after the window', () => {
  const now = 10_000_000;
  assert.equal(inSharedSession([line(now - SHARED_SEEN_MS - 1, src), line(now - 1000)], now), false);
  assert.equal(inSharedSession([], 5), false);
  assert.equal(inSharedSession([line(4), line(5)], 6), false);
});

test('V108-N12 only this channel drops foreign lines and leaves the room alone', () => {
  const all = [line(1), line(2, src), line(3)];
  assert.deepEqual(onlyHere(all).map((m) => m.ts), [1, 3]);
  assert.equal(all.length, 3);
  assert.equal(hiddenCount(all), 1);
});
