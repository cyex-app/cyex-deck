import assert from 'node:assert/strict';
import { test } from 'node:test';

import { StreamLoader } from './fmp4Stream.ts';
import {
  beforeStart,
  behindLive,
  broadcastLatency,
  canGoLive,
  DRIFT_GAIN,
  addAhead,
  EDGE_MIN,
  EDGE_SPAN,
  edgeJump,
  liveGain,
  shiftAheads,
  chatBarText,
  CLOCK_SLACK,
  dvrConfig,
  dvrTarget,
  edgeAt,
  liveConfig,
  LIVE_SNAP,
  liveState,
  maxLatency,
  needsLiveSeek,
  NO_EDGE_PULL,
  pausedBehind,
  PAUSE_STOP_MS,
  rangeEnd,
  replayChoice,
  resumePlan,
  sameStart,
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

test('V110-U62 a bar seek before the replay’s start is told apart from one at it', () => {
  assert.equal(beforeStart(10, 100), true);
  assert.equal(beforeStart(99.5, 100), false);
  assert.equal(beforeStart(100, 100), false);
  assert.equal(beforeStart(-5, 0), false, 'Twitch’s rewind starts at 0');
});

test('V108-N24 a replay starts where it was recorded', () => {
  assert.equal(dvrTarget(10, 400, 100), 100);
  assert.equal(dvrTarget(250, 400, 100), 250);
  assert.equal(dvrTarget(390, 400, 100), 'live');
  assert.deepEqual(stepTarget(110, -30, 400, 100), { at: 100, edge: 'start' });
  assert.deepEqual(stepTarget(200, -30, 400, 100), { at: 170, edge: null });
  // Not more than the live gap recorded yet: nothing to go back to.
  assert.deepEqual(stepTarget(400, -10, 400, 385), { at: 'live', edge: 'start' });
});

test('V108-N24 replay lengths', () => {
  assert.equal(replayChoice(5), 5);
  assert.equal(replayChoice(0), 0);
  assert.equal(replayChoice(7), 5);
  assert.equal(replayChoice(29), 30);
  assert.equal(replayChoice(200), 30);
});

test('live config is today’s, without the dead catch-up rate', () => {
  assert.deepEqual(liveConfig(false), { liveSyncDuration: 8, liveMaxLatencyDuration: 20, backBufferLength: 10, maxBufferLength: 12, maxMaxBufferLength: 20 });
  assert.equal(liveConfig(true).liveSyncDuration, 5);
  assert.equal(liveConfig(true).liveMaxLatencyDuration, 12);
  assert.equal('maxLiveSyncPlaybackRate' in liveConfig(false), false);
});

test('V108-LL3 low latency streams the segment being made (V108-LL7 all of it, 32 KB pieces)', () => {
  assert.equal(liveConfig(true).progressive, true);
  assert.equal(liveConfig(true).fLoader, StreamLoader);
  assert.equal('progressive' in liveConfig(false), false);
  assert.equal('fLoader' in liveConfig(false), false);
});

test('rewind config', () => {
  const c = dvrConfig(42);
  assert.equal(c.startPosition, 42);
  assert.ok((c.liveMaxLatencyDuration ?? 0) > 1e8);
  assert.ok((c.liveMaxLatencyDuration ?? 0) > (c.liveSyncDuration ?? Infinity));
});

test('what play does after a pause', () => {
  // 2026/10/31 Any pause at live, however short, comes back fully live.
  assert.equal(resumePlan({ pausedMs: 0, inVod: false, stopped: false }), 'sync');
  assert.equal(resumePlan({ pausedMs: 500, inVod: false, stopped: false }), 'sync');
  assert.equal(resumePlan({ pausedMs: 2000, inVod: false, stopped: false }), 'sync');
  assert.equal(resumePlan({ pausedMs: 30_000, inVod: false, stopped: false }), 'sync');
  assert.equal(resumePlan({ pausedMs: PAUSE_STOP_MS - 1, inVod: false, stopped: false }), 'sync');
  assert.equal(resumePlan({ pausedMs: PAUSE_STOP_MS, inVod: false, stopped: false }), 'reopen');
  assert.equal(resumePlan({ pausedMs: 500, inVod: false, stopped: true }), 'reopen');
  assert.equal(resumePlan({ pausedMs: 5000, inVod: true, stopped: false }), 'stay');
  assert.equal(resumePlan({ pausedMs: PAUSE_STOP_MS, inVod: true, stopped: true }), 'reload');
  // A deliberate rewind (Twitch's or the N24 replay) stays where it was, however long.
  assert.equal(resumePlan({ pausedMs: 500, inVod: true, stopped: false }), 'stay');
  assert.equal(resumePlan({ pausedMs: PAUSE_STOP_MS - 1, inVod: true, stopped: false }), 'stay');
});

test('going live moves the play head unless it is already there', () => {
  assert.equal(needsLiveSeek(100, 100), false);
  assert.equal(needsLiveSeek(100, 100 + 1), false);
  assert.equal(needsLiveSeek(100, 100 - LIVE_SNAP / 2), false);
  assert.equal(needsLiveSeek(100, 100 - LIVE_SNAP * 1.5), true);
  // 2026/10/31 Under 1 s behind used to stay behind.
  assert.equal(needsLiveSeek(100, 99.2), true);
  assert.equal(needsLiveSeek(100, 68.4), true);
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

test("V108-LL4 latency from the playing frame's date", () => {
  const now = 1_700_000_000_000;
  assert.equal(broadcastLatency(now, now - 1380), 1.38);
  assert.equal(broadcastLatency(now, now), 0);
  // No program date time: nothing to show.
  assert.equal(broadcastLatency(now, null), null);
  assert.equal(broadcastLatency(now, undefined), null);
  assert.equal(broadcastLatency(now, Number.NaN), null);
  // A local clock a little behind reads 0; far behind is not a latency.
  assert.equal(broadcastLatency(now, now + CLOCK_SLACK * 1000 - 100), 0);
  assert.equal(broadcastLatency(now, now + CLOCK_SLACK * 1000 + 100), null);
});

test('V109-F72 the bar’s live point counts to the ms; the same start written two ways is no restart', () => {
  const t0 = Date.parse('2026-11-07T10:00:00Z');
  assert.equal(edgeAt('2026-11-07T10:00:00Z', t0 + 1250), 1.25);
  assert.equal(uptimeAt('2026-11-07T10:00:00Z', t0 + 1250), 1, 'the clock stays whole seconds');
  assert.equal(edgeAt(null, t0), 0);
  assert.equal(edgeAt('2026-11-07T10:00:00Z', t0 - 5000), 0);
  assert.equal(sameStart('2026-11-07T10:00:00Z', '2026-11-07T10:00:00.000+00:00'), true);
  assert.equal(sameStart('2026-11-07T10:00:00Z', '2026-11-07T11:00:00+01:00'), true);
  assert.equal(sameStart('2026-11-07T10:00:00Z', '2026-11-07T10:05:00Z'), false);
  assert.equal(sameStart('x', 'x'), true);
  assert.equal(sameStart('x', 'y'), false);
});

test('V111-32 a Live press moves forward by the least buffer over the readings, less the spare', () => {
  // Whole 2 s segments: the buffer swings 0,6 → 2,6 s.
  const swing = Array.from({ length: 16 }, (_, i) => ({ at: i * 0.25, ahead: 2.6 - ((i * 0.25) % 2) }));
  assert.equal(edgeJump(swing, 0.9), null, 'the trough (0,85 s) is at the spare: nothing to gain');
  assert.ok(Math.abs(edgeJump(swing.map((x) => ({ ...x, ahead: x.ahead + 2 })), 0.9)! - 1.95) < 1e-9, 'trough 2,85 s: 1,95 s forward');
  const steady = Array.from({ length: 16 }, (_, i) => ({ at: i * 0.25, ahead: 3 + (i % 2) * 0.4 }));
  assert.ok(Math.abs(edgeJump(steady, 0.9)! - 2.1) < 1e-9, '3 s surely spare, 0,9 kept: 2,1 s forward');
  assert.equal(edgeJump(steady.slice(0, 8), 0.9), null, 'under EDGE_SPAN of readings: not yet');
  assert.equal(edgeJump([{ at: 0, ahead: 0.9 + EDGE_MIN - 0.01 }, { at: EDGE_SPAN, ahead: 2 }], 0.9), null, 'under EDGE_MIN: a seek only stutters');
  assert.ok(Math.abs(edgeJump([{ at: 0, ahead: 1.5 }, { at: EDGE_SPAN, ahead: 5 }], 0.9)! - 0.6) < 1e-9, "the least reading counts");
  assert.equal(edgeJump([], 0.9), null);
  assert.equal(edgeJump([{ at: 0, ahead: 3 }, { at: EDGE_SPAN, ahead: 3 }], -1), 3, 'a negative spare counts as 0');
});

test('V111-32 readings: old ones drop, a jump inside the buffer takes its length off', () => {
  let a = addAhead([], 0, 2);
  a = addAhead(a, 2, 3);
  a = addAhead(a, 10, 4);
  assert.deepEqual(a, [{ at: 10, ahead: 4 }]);
  assert.deepEqual(shiftAheads([{ at: 1, ahead: 3 }], 2), [{ at: 1, ahead: 1 }]);
});

test('V111-32 what a press gains: behind the live point, or (low latency) spare buffer', () => {
  assert.equal(liveGain(null, null), 0);
  assert.equal(liveGain(3, null), 3);
  assert.equal(liveGain(0.2, 2.1), 2.1);
  assert.equal(liveGain(3, 0.5), 3);
});

test('V111-32 the Live button works while "Live" once it drifted DRIFT_GAIN behind', () => {
  assert.equal(canGoLive(false, 0), true);
  assert.equal(canGoLive(true, 0), false);
  assert.equal(canGoLive(true, DRIFT_GAIN - 0.1), false);
  assert.equal(canGoLive(true, DRIFT_GAIN), true);
});
