// Run with `npm run test:ui` (Node's built-in test runner, no extra packages).

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { formatDate, formatDuration, formatTime, parseDuration } from './format.ts';

const at = new Date(2026, 9, 9, 14, 3, 7);
const morning = new Date(2026, 0, 2, 0, 5, 0);

test('dates are yyyy/mm/dd (3.1)', () => {
  assert.equal(formatDate(at), '2026/10/09');
  assert.equal(formatDate(morning), '2026/01/02');
});

test('times are 24-hour by default (3.2)', () => {
  assert.equal(formatTime(at), '14:03');
  assert.equal(formatTime(at, { seconds: true }), '14:03:07');
  assert.equal(formatTime(morning), '00:05');
});

test('12-hour timestamps when chosen (2.10)', () => {
  assert.equal(formatTime(at, { hour12: true }), '2:03 pm');
  assert.equal(formatTime(morning, { hour12: true, seconds: true }), '12:05:00 am');
});

test('durations format to two units at most', () => {
  assert.equal(formatDuration(60), '1m');
  assert.equal(formatDuration(90), '1m 30s');
  assert.equal(formatDuration(86_400), '1d');
  assert.equal(formatDuration(1_209_600), '2w');
  assert.equal(formatDuration(0), '0s');
});

test('durations parse from short forms', () => {
  assert.equal(parseDuration('30s'), 30);
  assert.equal(parseDuration('10m'), 600);
  assert.equal(parseDuration('1h 30m'), 5400);
  assert.equal(parseDuration('2D'), 172_800);
  assert.equal(parseDuration('45'), 45);
});

test('unreadable durations are rejected', () => {
  for (const bad of ['', 'abc', '10x', '5m later', '0m']) {
    assert.equal(parseDuration(bad), null, bad);
  }
});

test('format and parse round trip', () => {
  for (const secs of [60, 600, 3600, 86_400, 5400]) {
    assert.equal(parseDuration(formatDuration(secs)), secs);
  }
});
