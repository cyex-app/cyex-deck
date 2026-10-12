import assert from 'node:assert/strict';
import { test } from 'node:test';

import { nextHeld, playingNow } from './hold.ts';

// a plays; b is chat only; c is offline; d shows a live stream.
const playing = (t: string) => t === 'a';
const liveViewer = (t: string) => t === 'd';
const go = (held: string | null, from: string | null, to: string | null) => nextHeld({ held, from, to, playing, liveViewer });

test('a stream keeps playing behind a chat-only or offline tab (V102-15)', () => {
  assert.equal(go(null, 'a', 'b'), 'a');
  assert.equal(go(null, 'a', 'c'), 'a');
  assert.equal(go(null, 'a', null), 'a', 'home or settings too');
  assert.equal(go('a', 'b', 'c'), 'a', 'from one quiet tab to the next');
});

test('a tab with a live stream on screen stops it', () => {
  assert.equal(go(null, 'a', 'd'), null);
  assert.equal(go('a', 'b', 'd'), null);
});

test('back on the tab it is simply on screen again', () => {
  assert.equal(go('a', 'b', 'a'), null);
});

test('a muted stream counts as playing; a paused or ended one does not', () => {
  const v = (paused: boolean, ended = false, muted = false) => ({ paused, ended, muted });
  assert.equal(playingNow([v(false, false, true)]), true);
  assert.equal(playingNow([v(true), v(false, true)]), false);
  assert.equal(playingNow([]), false);
});

test('nothing was playing: nothing is kept', () => {
  assert.equal(go(null, 'b', 'c'), null);
  assert.equal(go(null, null, 'b'), null);
});
