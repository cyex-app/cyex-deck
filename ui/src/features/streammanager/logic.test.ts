import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ActivityFilters } from '../../ipc/bindings/ActivityFilters';
import type { SmActivity } from '../../ipc/bindings/SmActivity';
import { adIn, adIntervals, adLengths, alertAction, capPush, raidCountdown, shares, visibleActivity } from './logic.ts';

const ALL: ActivityFilters = {
  follows: true,
  subs: true,
  gifts: true,
  cheers: true,
  raids: true,
  redemptions: true,
  streaks: true,
  powerUps: true,
  hypeTrains: true,
  goals: true,
};

function item(id: string, kind: string): SmActivity {
  return { id, kind, text: id, user: null, ts: 0, alert: null, status: null };
}

test('alerts offer skip while waiting or playing, else replay (D4)', () => {
  assert.equal(alertAction('QUEUED'), 'skip');
  assert.equal(alertAction('PLAYING'), 'skip');
  assert.equal(alertAction('PLAYED'), 'replay');
  assert.equal(alertAction('SKIPPED'), 'replay');
  assert.equal(alertAction(null), null);
});

test('the raid countdown opens Raid now after 10 s (D2)', () => {
  assert.deepEqual(raidCountdown(1000, 1090, 1005), { left: 85, ready: false });
  assert.deepEqual(raidCountdown(1000, 1090, 1010.5), { left: 80, ready: true });
  assert.deepEqual(raidCountdown(1000, 1090, 2000), { left: 0, ready: true });
});

test('ad settings offer what Twitch allows (D5)', () => {
  assert.deepEqual(adLengths(30, 180, 90), [30, 60, 90, 120, 150, 180]);
  assert.deepEqual(adLengths(30, 90, 45), [30, 45, 60, 90]);
  assert.deepEqual(adIntervals(480, 3600, 1800), [480, 600, 900, 1200, 1500, 1800, 2400, 2700, 3000, 3600]);
  assert.deepEqual(adIntervals(900, 1800, 1000), [900, 1000, 1200, 1500, 1800]);
});

test('activity is newest first and filtered by kind', () => {
  const items = [item('a', 'follow'), item('b', 'sub'), item('c', 'cheer'), item('d', 'mystery')];
  assert.deepEqual(visibleActivity(items, ALL).map((i) => i.id), ['d', 'c', 'b', 'a']);
  const noFollows = { ...ALL, follows: false, cheers: false };
  assert.deepEqual(visibleActivity(items, noFollows).map((i) => i.id), ['d', 'b']);
  // The input stays as it was.
  assert.equal(items[0].id, 'a');
});

test('capPush keeps the newest', () => {
  assert.deepEqual(capPush([1, 2, 3], 4, 3), [2, 3, 4]);
  assert.deepEqual(capPush([], 1, 3), [1]);
});

test('ad countdown', () => {
  const now = Date.parse('2026-10-10T12:00:00Z');
  assert.equal(adIn('2026-10-10T12:05:00Z', now), 300);
  assert.equal(adIn('2026-10-10T11:00:00Z', now), 0);
  assert.equal(adIn('', now), null);
  assert.equal(adIn('nope', now), null);
});

test('prediction shares', () => {
  assert.deepEqual(shares([300, 100]), [75, 25]);
  assert.deepEqual(shares([0, 0]), [0, 0]);
});
