import assert from 'node:assert/strict';
import { test } from 'node:test';
import { pinSpan } from './pinTimer.ts';

const T = Date.parse('2026-10-11T12:00:00Z');
const at = (ms: number) => new Date(T + ms).toISOString();

test('a pin without an end has no timer', () => {
  assert.equal(pinSpan({ startsAt: at(0), endsAt: null }, T, T), null);
  assert.equal(pinSpan({ startsAt: null, endsAt: 'junk' }, T, T), null);
});

test('counts from the start Twitch sent', () => {
  const s = pinSpan({ startsAt: at(0), endsAt: at(60_000) }, T + 5000, T + 15_000)!;
  assert.equal(s.left, 45_000);
  assert.equal(s.frac, 0.75);
});

test('falls back to when the app saw the pin', () => {
  const s = pinSpan({ startsAt: null, endsAt: at(30_000) }, T + 10_000, T + 20_000)!;
  assert.equal(s.left, 10_000);
  assert.equal(s.frac, 0.5);
});

test('over, or seen late, stays inside 0..1', () => {
  assert.deepEqual(pinSpan({ startsAt: null, endsAt: at(1000) }, T, T + 5000), { left: 0, frac: 0 });
  assert.equal(pinSpan({ startsAt: at(50_000), endsAt: at(60_000) }, T, T)!.frac, 1);
});
