import assert from 'node:assert/strict';
import { test } from 'node:test';

import { interleave } from './pcm.ts';

const buf = (...channels: number[][]) => ({
  numberOfChannels: channels.length,
  length: channels[0].length,
  getChannelData: (i: number) => Float32Array.from(channels[i]),
});

test('stereo goes left, right, left, right', () => {
  assert.deepEqual([...interleave(buf([1, 2], [3, 4]), 10)], [1, 3, 2, 4]);
});

test('mono plays on both sides', () => {
  assert.deepEqual([...interleave(buf([0.5, -0.5]), 10)], [0.5, 0.5, -0.5, -0.5]);
});

test('long sounds are cut', () => {
  assert.equal(interleave(buf([1, 2, 3]), 2).length, 4);
});
