import assert from 'node:assert/strict';
import { test } from 'node:test';

import { CARDS, fitTile, HYSTERESIS, MIN, stackColumns, tileNeed } from './fit.ts';

test('a roomy tile keeps the user layout', () => {
  assert.deepEqual(fitTile({ w: 1600, h: 900 }, 'right', 0.28), { side: 'right', ratio: 0.28, moved: false });
  assert.deepEqual(fitTile({ w: 1600, h: 900 }, 'top', 0.3), { side: 'top', ratio: 0.3, moved: false });
});

test('a narrow chat is widened before anything moves', () => {
  // 0.28 of 700 is 196 px: the chat gets its 240, the stream keeps 460.
  const f = fitTile({ w: 700, h: 500 }, 'right', 0.28);
  assert.equal(f.side, 'right');
  assert.equal(f.ratio, MIN.chatW / 700);
});

test('too narrow for both side by side: chat goes below the stream', () => {
  const f = fitTile({ w: 420, h: 700 }, 'left', 0.28);
  assert.equal(f.side, 'bottom');
  assert.ok(f.moved);
  // The stream keeps its 16:9 height at full width, the chat gets the rest.
  assert.equal(Math.round(f.ratio * 700), 700 - Math.round((420 * 9) / 16));
});

test('a moved layout waits for some extra room before it goes back', () => {
  const edge = MIN.chatW + MIN.streamW;
  const moved = fitTile({ w: edge - 1, h: 700 }, 'right', 0.28);
  assert.ok(moved.moved);
  assert.ok(fitTile({ w: edge + 10, h: 700 }, 'right', 0.28, true).moved, 'within the hysteresis it stays');
  assert.equal(fitTile({ w: edge + HYSTERESIS, h: 700 }, 'right', 0.28, true).moved, false);
  // Not moved yet: the same width keeps the user layout.
  assert.equal(fitTile({ w: edge + 10, h: 700 }, 'right', 0.28, false).moved, false);
});

test('a chat below a short stream goes beside it', () => {
  const f = fitTile({ w: 1200, h: 300 }, 'bottom', 0.28);
  assert.deepEqual([f.side, f.moved], ['right', true]);
});

test('nothing fits: the user side, the stream keeps its minimum', () => {
  const f = fitTile({ w: 400, h: 250 }, 'right', 0.28);
  assert.deepEqual([f.side, f.moved], ['right', false]);
  assert.equal(Math.round(400 * (1 - f.ratio)), MIN.streamW);
});

test('a hidden tile (no size) keeps what it had', () => {
  assert.deepEqual(fitTile({ w: 0, h: 0 }, 'left', 0.4), { side: 'left', ratio: 0.4, moved: false });
});

test('narrow columns of chats stack, and come back with room to spare', () => {
  const chat = tileNeed(false, true);
  const cols = (n: number) => Array.from({ length: n }, () => ({ size: 1, tiles: [chat] }));
  assert.equal(stackColumns({ w: 1200, h: 800 }, cols(3)), false);
  assert.equal(stackColumns({ w: 600, h: 800 }, cols(3)), true, '200 px chats stack');
  assert.equal(stackColumns({ w: 3 * MIN.chatW + 10, h: 800 }, cols(3), true), true, 'within the hysteresis');
  assert.equal(stackColumns({ w: 3 * (MIN.chatW + HYSTERESIS), h: 800 }, cols(3), true), false);
  // Too short to stack them all: stays in columns.
  assert.equal(stackColumns({ w: 600, h: 500 }, cols(3)), false);
  assert.equal(stackColumns({ w: 600, h: 800 }, cols(1)), false, 'one column has nothing to stack');
});

test('the stream manager chat goes under the cards when both do not fit side by side (V102-22)', () => {
  assert.deepEqual(fitTile({ w: 1000, h: 700 }, 'right', 0.3, false, CARDS), { side: 'right', ratio: 0.3, moved: false });
  const f = fitTile({ w: 500, h: 700 }, 'right', 0.3, false, CARDS);
  assert.deepEqual([f.side, f.moved, f.ratio], ['bottom', true, 0.45]);
});
