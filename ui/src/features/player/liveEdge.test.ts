import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  behindLive,
  chatBarText,
  dvrConfig,
  dvrTarget,
  liveConfig,
  liveState,
  PAUSE_STOP_MS,
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
