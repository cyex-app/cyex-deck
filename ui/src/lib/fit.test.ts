import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  CARDS,
  fitModview,
  fitTile,
  HYSTERESIS,
  MIN,
  type ModArea,
  MODVIEW,
  MODVIEW_SHARE,
  stackColumns,
  STREAM_HEAD,
  tileNeed,
} from './fit.ts';

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

// V102-26 The mod view inside a tab.
const area = (w: number, h: number, chat: ModArea['chat'] = null, partner: ModArea['partner'] = 'stream', share = MODVIEW_SHARE) => ({
  tile: { w, h },
  chat,
  partner,
  share,
});

test('chat under a wide stream: the mod view on the left half of the stream area', () => {
  const f = fitModview(area(2000, 1050, { side: 'bottom', ratio: 0.4 }));
  assert.deepEqual(f, { row: true, share: 0.5, hideChat: false, out: false });
});

test('chat beside a taller stream area: stream on top, the mod view below', () => {
  const f = fitModview(area(2000, 1050, { side: 'right', ratio: 0.28 }));
  assert.deepEqual(f, { row: false, share: 0.5, hideChat: false, out: false });
});

test('a stream-only tile splits by its shape, turning only past the hysteresis', () => {
  assert.equal(fitModview(area(1200, 700)).row, true);
  // About 20 px narrower than 16:9 under the header.
  const h = Math.round(((1000 + 20) * 9) / 16) + STREAM_HEAD;
  assert.equal(fitModview(area(1000, h)).row, false, 'taller than 16:9: top and bottom');
  const side = { row: true, share: 0.5, hideChat: false, out: false };
  assert.equal(fitModview(area(1000, h), side).row, true, 'within the hysteresis it stays side by side');
  const far = Math.round(((1000 + HYSTERESIS + 10) * 9) / 16) + STREAM_HEAD;
  assert.equal(fitModview(area(1000, far), side).row, false);
});

test('a dragged share keeps both panels at their minimum', () => {
  const f = fitModview(area(1000, 400, null, 'stream', 0.9));
  assert.equal(f.row, true);
  assert.equal(f.share, 1 - MIN.streamW / 1000);
  assert.equal(fitModview(area(1000, 400, null, 'stream', 0.1)).share, MIN.modviewW / 1000);
});

test('no room beside the chat: the chat steps aside, and needs extra room to come back', () => {
  const chat = { side: 'right' as const, ratio: MIN.chatW / 600 };
  const f = fitModview(area(600, 420, chat));
  assert.deepEqual([f.row, f.hideChat, f.out], [true, true, false]);
  // Just enough for the mod view next to the chat again: only once the chat wasn't hidden.
  const h = MIN.modviewH + MIN.streamH + STREAM_HEAD;
  const wide = { side: 'right' as const, ratio: MIN.chatW / (MIN.chatW + 400) };
  assert.equal(fitModview(area(MIN.chatW + 400, h + 10, wide)).hideChat, false);
  assert.equal(fitModview(area(MIN.chatW + 400, h + 10, wide), { ...f, row: false }).hideChat, true, 'within the hysteresis');
  assert.equal(fitModview(area(MIN.chatW + 400, h + HYSTERESIS, wide), { ...f, row: false }).hideChat, false);
});

test('no room even without the chat: it pops out', () => {
  const f = fitModview(area(400, 300, { side: 'bottom', ratio: 0.4 }));
  assert.deepEqual([f.out, f.hideChat], [true, false]);
  assert.equal(fitModview(area(MIN.modviewW + MIN.streamW - 1, MIN.modviewH + MIN.streamH + STREAM_HEAD - 1)).out, true);
  assert.equal(fitModview(area(MIN.modviewW + MIN.streamW, MIN.modviewH + STREAM_HEAD)).out, false, 'side by side just fits');
});

test('beside a chat with no stream it takes the whole rest', () => {
  assert.deepEqual(fitModview(area(900, 600, { side: 'right', ratio: 0.3 }, null)), { row: true, share: 1, hideChat: false, out: false });
  assert.equal(fitModview(area(500, 600, { side: 'right', ratio: 0.5 }, null)).hideChat, true);
  // The chat goes under the mod view before that (V102-24 with the mod view's size).
  assert.equal(fitTile({ w: 500, h: 700 }, 'right', 0.3, false, MODVIEW).side, 'bottom');
});
