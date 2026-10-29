import { test } from 'node:test';
import assert from 'node:assert/strict';
import { coverKey, covered, type Shown } from './streamerCover.ts';

const shown: Shown = { user: '42', source: '7tv', opening: 0 };
const key = coverKey(shown);

test('blurred while streamer mode hides pictures, until clicked', () => {
  assert.equal(covered(true, true, '', key), true);
  assert.equal(covered(true, true, key, key), false);
});

test('nothing to blur with streamer mode off or no picture', () => {
  assert.equal(covered(false, true, '', key), false);
  assert.equal(covered(true, false, '', key), false);
});

test('switching between the Twitch and 7TV picture blurs it again', () => {
  assert.equal(covered(true, true, key, coverKey({ ...shown, source: 'twitch' })), true);
});

test('opening the card again blurs it again', () => {
  assert.equal(covered(true, true, key, coverKey({ ...shown, opening: 1 })), true);
});

test('another user blurs it again', () => {
  assert.equal(covered(true, true, key, coverKey({ ...shown, user: '43' })), true);
});
