import assert from 'node:assert/strict';
import { test } from 'node:test';

import { BAR_SLACK, dateAt, goneDue, isGone, nextAfter, type Piece, posAt, replayBarStart, replayStart } from './replayClock.ts';

const T0 = Date.parse('2026-10-29T12:00:00Z');

/** 2 s segments from playlist second 0; the fourth comes after a 60 s hole. */
const pieces: Piece[] = [
  { sn: 10, start: 0, duration: 2, programDateTime: T0 },
  { sn: 11, start: 2, duration: 2, programDateTime: T0 + 2000 },
  { sn: 12, start: 4, duration: 2, programDateTime: T0 + 4000 },
  { sn: 13, start: 6, duration: 2, programDateTime: T0 + 66_000 },
];

test('date at a playlist position', () => {
  assert.equal(dateAt(pieces, 0), T0);
  assert.equal(dateAt(pieces, 3.5), T0 + 3500);
  assert.equal(dateAt(pieces, 7), T0 + 67_000);
  assert.equal(dateAt(pieces, 99), T0 + 68_000, 'past the end: the last segment’s end');
  assert.equal(dateAt([], 1), null);
  assert.equal(dateAt([{ sn: 1, start: 0, duration: 2, programDateTime: null }], 1), null);
});

test('playlist position of a date', () => {
  assert.equal(posAt(pieces, T0 + 1000), 1);
  assert.equal(posAt(pieces, T0 + 5000), 5);
  assert.equal(posAt(pieces, T0 + 30_000), 6, 'in the hole: the next piece');
  assert.equal(posAt(pieces, T0 - 9000), 0);
  assert.equal(posAt(pieces, T0 + 999_000), 6, 'past the end: the last segment');
  assert.equal(posAt([], T0), null);
});

test('init segments and missing dates are skipped', () => {
  const init: Piece = { sn: 'initSegment', start: 0, duration: 0, programDateTime: T0 - 1 };
  assert.equal(posAt([init, ...pieces], T0 + 1000), 1);
  assert.equal(dateAt([init, ...pieces], 0), T0);
});

test('V108-N24b the replay starts once, where the playlist it read has the spot', () => {
  // The offset (5 s) was read with sn 10 still in the ring; the playlist hls.js got starts at
  // sn 11. Placed here, before anything loads: no second startLoad (that one left hls.js stuck).
  const trimmed = pieces.slice(1).map((p) => ({ ...p, start: p.start - 2 }));
  assert.equal(replayStart(trimmed, T0 + 5000, 5), 3);
  assert.equal(replayStart(pieces, T0 + 5000, 5), 5, 'nothing trimmed: the same spot');
  assert.equal(replayStart(undefined, T0 + 5000, 5), 5, 'no segments read: the offset');
  assert.equal(replayStart([{ sn: 1, start: 0, duration: 2, programDateTime: null }], T0, 7), 7, 'no dates: the offset');
});

test('the next segment after a missing one', () => {
  const origin = T0 - 600_000;
  assert.equal(nextAfter(pieces, 11, origin), 604);
  assert.equal(nextAfter(pieces, 12, origin), 666);
  assert.equal(nextAfter(pieces, 13, origin), null);
});

test('a missing segment ahead waits for the play head', () => {
  // Buffered 10..40, the missing one starts at 40: 30 s still play.
  assert.equal(goneDue(12, 40, 40), false);
  assert.equal(goneDue(39.8, 40, 40), true, 'there');
  assert.equal(goneDue(20, 20.2, 40), true, 'out of buffer before it');
  assert.equal(goneDue(20, null, 40), true, 'not in any buffer');
});

test('only 404 and 410 mean gone', () => {
  assert.equal(isGone(404), true);
  assert.equal(isGone(410), true);
  assert.equal(isGone(403), false);
  assert.equal(isGone(429), false);
  assert.equal(isGone(0), false);
});

test('V109-F72 the full replay bar slides with the clock, not with each trimmed segment', () => {
  // A 5 min ring of 2 s segments, trimmed as replay.rs does, its range read every 2 s, the bar
  // drawn four times a second while the play head plays 60 s behind live. A moment's tick sits
  // at stream second 500.
  const keep = 300;
  const latency = 3;
  const tick = 500;
  let from = 0;
  let read = from;
  let last: { start: number; thumb: number; tick: number; was: number } | null = null;
  let hop = 0;
  let oldHop = 0;
  for (let i = 0; i < 4 * 600; i++) {
    const edge = 400 + i / 4;
    const newest = Math.floor((edge - latency) / 2) * 2;
    while (newest - from >= keep) from += 2;
    if (i % 8 === 0) read = from;
    const start = replayBarStart(read, edge, keep);
    const at = (t: number, s: number) => (t - s) / (edge - s);
    const now = { start, thumb: at(edge - 60, start), tick: at(tick, start), was: at(tick, read) };
    if (last && tick > start) {
      assert.ok(start >= last.start, `the start never goes back (${start} < ${last.start})`);
      assert.ok(start - last.start <= 0.25 + 1e-9, `the start moves with the clock (${start - last.start} at ${edge})`);
      hop = Math.max(hop, Math.abs(now.thumb - last.thumb), Math.abs(now.tick - last.tick));
      // Before: the bar started at what the ring held, read every 2 s.
      oldHop = Math.max(oldHop, Math.abs(now.was - last.was));
    }
    assert.ok(start >= read, 'nothing on the bar before what the replay holds');
    last = now;
  }
  assert.ok(hop < 0.0015, `thumb and ticks glide (${hop})`);
  assert.ok(oldHop > 0.004, `the old bar's ticks hopped (${oldHop})`);
});

test('V109-F72 a replay still filling starts at its oldest second', () => {
  assert.equal(replayBarStart(50, 120, 300), 50);
  assert.equal(replayBarStart(50, 120, 0), 50, 'no length known: what it holds');
  assert.equal(replayBarStart(50, 400, 300), 400 - 300 + BAR_SLACK);
});
