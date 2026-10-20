import assert from 'node:assert/strict';
import { test } from 'node:test';

import { ENTRIES } from './registry/index.ts';
import { pageResetPatch } from './resetPatch.ts';

test('t5 a page reset keeps what other pages show', () => {
  const general = pageResetPatch(ENTRIES, 'general', 'general');
  assert.equal(general.chatFontSize, null);
  assert.equal(general.chatPosition, null);
  assert.equal('sound' in general, false, 'Sound has its own page');
  assert.equal('streamer' in general, false, 'Streamer mode has its own page');

  const chat = pageResetPatch(ENTRIES, 'chat', 'chat');
  assert.deepEqual((chat.timestamps as Record<string, unknown>).show, null);
  assert.equal('badges' in chat, false, 'badges are on Emotes and badges');
  assert.equal('users' in chat, false, 'nicknames and notes stay');
  assert.equal('sharePresence' in chat, false);
});

test('t5 a parent reset wins over its children', () => {
  const entries = [
    { id: 'p.a.b', page: 'x', label: 'b' },
    { id: 'p.a', page: 'x', label: 'a' },
    { id: 'p.a.c', page: 'x', label: 'c' },
    { id: 'p.d', page: 'y', label: 'd' },
    { id: 'q.e', page: 'x', label: 'e' },
  ] as const;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  assert.deepEqual(pageResetPatch(entries as any, 'p', 'x'), { a: null });
});
