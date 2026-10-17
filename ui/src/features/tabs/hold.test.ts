import assert from 'node:assert/strict';
import { test } from 'node:test';

import { heldAfterStart, nextHeld, playingNow } from './hold.ts';

// a plays; b is chat only; c is offline; d shows a live stream; p is live in its own window.
const playing = (t: string) => t === 'a';
const playsElsewhere = (t: string) => t === 'p';
const go = (held: string | null, from: string | null, to: string | null) => nextHeld({ held, from, to, playing, playsElsewhere });

test('a stream keeps playing behind a chat-only or offline tab (V102-15)', () => {
  assert.equal(go(null, 'a', 'b'), 'a');
  assert.equal(go(null, 'a', 'c'), 'a', 'an offline tab, new or not');
  assert.equal(go(null, 'a', null), 'a', 'home or settings too');
  assert.equal(go('a', 'b', 'c'), 'a', 'from one quiet tab to the next');
});

test('a live tab keeps the old stream until its own one plays', () => {
  assert.equal(go(null, 'a', 'd'), 'a');
  assert.equal(go('a', 'b', 'd'), 'a');
});

test('a tab live in its own window stops it at once', () => {
  assert.equal(go(null, 'a', 'p'), null);
  assert.equal(go('a', 'b', 'p'), null);
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

const start = (held: string | null, shown: string | null, tab: string | null, live = true) => heldAfterStart({ held, shown, tab, live });

test('the live stream on screen starting stops the held one', () => {
  assert.equal(start('a', 'd', 'd'), null);
  assert.equal(start('a', 'c', 'c'), null, 'an offline tab that went live later');
});

test('a VOD, clip or story on screen leaves it playing', () => {
  assert.equal(start('a', 'c', 'c', false), 'a');
});

test('the held tab itself or a view leaves it playing', () => {
  assert.equal(start('a', 'd', 'a'), 'a', 'the held stream picking up after a stall');
  assert.equal(start('a', null, 'a'), 'a');
  assert.equal(start('a', 'd', null), 'a', 'a video outside any tab');
});

test('nothing held: nothing changes', () => {
  assert.equal(start(null, 'd', 'd'), null);
});
