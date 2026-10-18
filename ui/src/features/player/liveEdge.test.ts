import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  behindLive,
  chatBarText,
  dvrConfig,
  dvrTarget,
  liveConfig,
  liveState,
  maxLatency,
  NO_EDGE_PULL,
  pausedBehind,
  PAUSE_STOP_MS,
  rangeEnd,
  resumePlan,
  stepTarget,
  uptimeAt,
} from './liveEdge.ts';

test('behind live', () => {
  assert.equal(behindLive(null, 10), null);
  assert.equal(behindLive(5, 10), 0);
  assert.equal(behindLive(30, 18), 12);
});

test('live state keeps still between 2 and 4 s', () => {
  assert.equal(liveState('live', 3), 'live');
  assert.equal(liveState('live', 4), 'live');
  assert.equal(liveState('live', 4.1), 'behind');
  assert.equal(liveState('behind', 3), 'behind');
  assert.equal(liveState('behind', 2), 'behind');
  assert.equal(liveState('behind', 1.9), 'live');
  assert.equal(liveState('live', null), 'live');
  assert.equal(liveState('behind', null), 'behind');
});

test('uptime', () => {
  const start = '2026-10-17T10:00:00Z';
  assert.equal(uptimeAt(null, Date.now()), 0);
  assert.equal(uptimeAt('not a date', Date.now()), 0);
  assert.equal(uptimeAt(start, Date.parse(start) + 90_900), 90);
  assert.equal(uptimeAt(start, Date.parse(start) - 5000), 0);
});

test('bar seek', () => {
  assert.equal(dvrTarget(79, 100), 79);
  assert.equal(dvrTarget(81, 100), 'live');
  assert.equal(dvrTarget(-5, 100), 0);
  assert.equal(dvrTarget(0, 10), 'live');
  assert.equal(dvrTarget(5, 10), 'live');
});

test('key steps', () => {
  assert.deepEqual(stepTarget(100, -10, 100), { at: 80, edge: null });
  assert.deepEqual(stepTarget(30, -40, 100), { at: 0, edge: 'start' });
  assert.deepEqual(stepTarget(0, -10, 100), { at: 0, edge: 'start' });
  assert.deepEqual(stepTarget(70, 15, 100), { at: 'live', edge: null });
  assert.deepEqual(stepTarget(15, -10, 15), { at: 'live', edge: 'start' });
  assert.deepEqual(stepTarget(50, 10, 100), { at: 60, edge: null });
});

test('live config is today’s, without the dead catch-up rate', () => {
  assert.deepEqual(liveConfig(false), { liveSyncDuration: 8, liveMaxLatencyDuration: 20, backBufferLength: 10, maxBufferLength: 12, maxMaxBufferLength: 20 });
  assert.equal(liveConfig(true).liveSyncDuration, 4);
  assert.equal(liveConfig(true).liveMaxLatencyDuration, 12);
  assert.equal('maxLiveSyncPlaybackRate' in liveConfig(false), false);
});

test('rewind config', () => {
  const c = dvrConfig(42);
  assert.equal(c.startPosition, 42);
  assert.ok((c.liveMaxLatencyDuration ?? 0) > 1e8);
  assert.ok((c.liveMaxLatencyDuration ?? 0) > (c.liveSyncDuration ?? Infinity));
});

test('what play does after a pause', () => {
  assert.equal(resumePlan({ pausedMs: 500, inVod: false, stopped: false }), 'play');
  assert.equal(resumePlan({ pausedMs: 5000, inVod: false, stopped: false }), 'sync');
  assert.equal(resumePlan({ pausedMs: PAUSE_STOP_MS, inVod: false, stopped: false }), 'reopen');
  assert.equal(resumePlan({ pausedMs: 500, inVod: false, stopped: true }), 'reopen');
  assert.equal(resumePlan({ pausedMs: 5000, inVod: true, stopped: false }), 'stay');
  assert.equal(resumePlan({ pausedMs: PAUSE_STOP_MS, inVod: true, stopped: true }), 'reload');
});

test('chat bar text', () => {
  assert.equal(chatBarText(134), 'Chat is live. The stream is 2:14 behind.');
});

test('U10 end of the buffered range holding a time', () => {
  const ranges = (...r: [number, number][]) => ({ length: r.length, start: (i: number) => r[i][0], end: (i: number) => r[i][1] });
  const r = ranges([0, 10], [20, 30]);
  assert.equal(rangeEnd(r, 5), 10);
  assert.equal(rangeEnd(r, 0), 10);
  assert.equal(rangeEnd(r, 30), 30);
  assert.equal(rangeEnd(r, 15), null);
  assert.equal(rangeEnd(r, 31), null);
  assert.equal(rangeEnd(ranges(), 0), null);
});

test('d-1 paused, hls.js never pulls the video to the edge', () => {
  assert.equal(maxLatency(false, false), 20);
  assert.equal(maxLatency(true, false), 12);
  assert.equal(maxLatency(true, true), NO_EDGE_PULL);
  assert.equal(maxLatency(false, true), NO_EDGE_PULL);
  // Hours behind still isn't past it.
  assert.ok(NO_EDGE_PULL > 10 * 3600);
  assert.equal(dvrConfig(0).liveMaxLatencyDuration, NO_EDGE_PULL);
});

test('d-1 paused at live, behind counts up from the pause', () => {
  assert.equal(pausedBehind(1.5, 0), 1.5);
  assert.equal(pausedBehind(1.5, 6000), 7.5);
  assert.equal(pausedBehind(1.5, 8000), 9.5);
  // Never back: the readout only grows while paused.
  const seq = [0, 2000, 6000, 8000, 14_000].map((ms) => pausedBehind(2, ms)!);
  for (let i = 1; i < seq.length; i++) assert.ok(seq[i] > seq[i - 1]);
  assert.equal(pausedBehind(null, 5000), null);
  assert.equal(pausedBehind(3, -50), 3);
});
