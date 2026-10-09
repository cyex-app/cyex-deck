import assert from 'node:assert/strict';
import { test } from 'node:test';

import { levelName, pickLevel } from './quality.ts';

// hls.js sorts levels by bitrate, lowest first.
const levels = [
  { height: 160, frameRate: 30, bitrate: 230_000 },
  { height: 360, frameRate: 30, bitrate: 630_000 },
  { height: 480, frameRate: 30, bitrate: 1_400_000 },
  { height: 720, frameRate: 60, bitrate: 3_400_000 },
  { height: 1080, frameRate: 60, bitrate: 8_500_000 },
];

test('auto fits the player', () => {
  assert.equal(pickLevel(levels, 'auto', 400), 2, '480p covers a 400 px player');
  assert.equal(pickLevel(levels, 'auto', 720), 3);
  assert.equal(pickLevel(levels, 'auto', 1400), 4, 'nothing covers it: the best');
});

test('fixed choices cap the height', () => {
  assert.equal(pickLevel(levels, 'source', 100), 4);
  assert.equal(pickLevel(levels, '720', 2000), 3);
  assert.equal(pickLevel(levels.slice(2), '360', 2000), 0, 'nothing that small: the lowest');
  assert.equal(pickLevel([], 'auto', 100), -1);
});

test('names like Twitch', () => {
  assert.equal(levelName(levels[4]), '1080p60');
  assert.equal(levelName(levels[2]), '480p');
});
