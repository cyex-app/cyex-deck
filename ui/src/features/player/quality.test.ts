import assert from 'node:assert/strict';
import { test } from 'node:test';

import { levelFor, levelName, lowestLevel, pickLevel, qualityOptions } from './quality.ts';

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

test('out of sight a minute: the lowest, then the choice again (V103-03)', () => {
  const withAudio = [{ height: 0, bitrate: 160_000 }, ...levels];
  assert.equal(lowestLevel(withAudio), 1, 'audio only is not a video level');
  assert.equal(lowestLevel([]), -1);
  assert.equal(levelFor(levels, '720', 2000, true), 0, 'a fixed choice drops too');
  assert.equal(levelFor(levels, '720', 2000, false), 3, 'and comes back as it was');
  assert.equal(levelFor(levels, 'auto', 400, true), 0);
  assert.equal(levelFor(levels, 'auto', 400, false), 2);
});

test('names like Twitch', () => {
  assert.equal(levelName(levels[4]), '1080p60');
  assert.equal(levelName(levels[2]), '480p');
});

test('the quality list: auto, levels best first, audio only', () => {
  const named = [
    { index: 4, name: '1080p60', height: 1080 },
    { index: 3, name: '720p60', height: 720 },
    { index: 0, name: '160p', height: 160 },
    { index: 5, name: 'Audio only', height: 0 },
  ];
  const list = qualityOptions(named, 3, 'auto');
  assert.deepEqual(
    list.map((o) => [o.label, o.hint, o.choice]),
    [
      ['Auto', '720p60', 'auto'],
      ['1080p60', 'Source', 'source'],
      ['720p60', undefined, '720'],
      ['160p', undefined, '160'],
      ['Audio only', undefined, 'audio'],
    ],
  );
  assert.equal(qualityOptions(named, 3, '720')[0].hint, undefined, 'a fixed choice shows no auto note');
});
