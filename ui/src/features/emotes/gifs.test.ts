import assert from 'node:assert/strict';
import { test } from 'node:test';

import { cooldownText, cooldownUntil, secsLeft, sendError, showGifTab, startCooldown } from './gifs.ts';

test('the tab shows only where this account can send', () => {
  assert.equal(showGifTab({ canSend: true, reason: null }, false), true);
  assert.equal(showGifTab({ canSend: false, reason: 'Subscribe at Tier 2 or 3 to send GIFs here.' }, false), false);
  assert.equal(showGifTab(null, false), false);
  assert.equal(showGifTab(undefined, false), false);
  // Emote-only can start while the menu is open.
  assert.equal(showGifTab({ canSend: true, reason: null }, true), false);
});

test('cooldown text counts whole seconds up', () => {
  assert.equal(secsLeft(10_000, 0), 10);
  assert.equal(secsLeft(10_000, 9_001), 1);
  assert.equal(secsLeft(10_000, 10_000), 0);
  assert.equal(secsLeft(10_000, 12_000), 0);
  assert.equal(cooldownText(24), 'Next GIF in 24 s');
});

test('send errors are short and a wait is a countdown, not an error', () => {
  assert.equal(sendError({ error: null, waitSecs: null }), null);
  assert.equal(sendError({ error: 'TEMPORARILY_UNAVAILABLE', waitSecs: null }), 'GIFs are unavailable right now.');
  assert.equal(sendError({ error: 'TEMPORARILY_UNAVAILABLE', waitSecs: 20 }), 'GIFs are unavailable right now.');
  assert.equal(sendError({ error: 'SOMETHING', waitSecs: 24 }), null);
  assert.equal(sendError({ error: 'SOMETHING', waitSecs: null }), 'Couldn’t send that GIF. Try again in a bit.');
});

test('a cooldown stays per chat', () => {
  assert.equal(cooldownUntil('a:x'), 0);
  assert.equal(startCooldown('a:x', { error: null, waitSecs: 30 }, 1_000), 31_000);
  assert.equal(cooldownUntil('a:x'), 31_000);
  assert.equal(cooldownUntil('a:y'), 0);
  // No wait given: the old end stays.
  assert.equal(startCooldown('a:x', { error: null, waitSecs: null }, 2_000), 31_000);
});
