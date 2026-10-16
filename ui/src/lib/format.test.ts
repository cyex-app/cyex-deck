// Run with `npm run test:ui` (Node's built-in test runner, no extra packages).

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { ago, formatBytes, formatDate, formatDuration, formatNumber, formatSecs, formatShort, formatTime, parseDuration } from './format.ts';

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

test('numbers group with dots (3.13)', () => {
  assert.equal(formatNumber(0), '0');
  assert.equal(formatNumber(999), '999');
  assert.equal(formatNumber(1000), '1.000');
  assert.equal(formatNumber(1_234_567), '1.234.567');
  assert.equal(formatNumber(-45_000), '-45.000');
});

test('short numbers round down with a comma decimal', () => {
  assert.equal(formatShort(950), '950');
  assert.equal(formatShort(1000), '1K');
  assert.equal(formatShort(1250), '1,2K');
  assert.equal(formatShort(999_999), '999,9K');
  assert.equal(formatShort(2_900_000), '2,9M');
  assert.equal(formatShort(3_000_000_000), '3B');
});

test('ago is short, then a date', () => {
  const now = at.getTime();
  assert.equal(ago(now - 30_000, now), 'now');
  assert.equal(ago(now - 5 * 60_000, now), '5m');
  assert.equal(ago(now - 3 * 3_600_000, now), '3h');
  assert.equal(ago(now - 2 * 86_400_000, now), '2d');
  assert.equal(ago(morning.getTime(), now), '2026/01/02');
});

test('formatBytes uses dots, commas and binary units', () => {
  assert.equal(formatBytes(0), '0 B');
  assert.equal(formatBytes(900), '900 B');
  assert.equal(formatBytes(38 * 1024 * 1024), '38 MB');
  assert.equal(formatBytes(1.5 * 1024 * 1024), '1,5 MB');
  assert.equal(formatBytes(1300 * 1024 * 1024), '1,3 GB');
  assert.equal(formatBytes(2048 * 1024 * 1024 * 1024), '2 TB');
});

test('compact durations', () => {
  assert.equal(formatSecs(30), '30s');
  assert.equal(formatSecs(60), '60s');
  assert.equal(formatSecs(180), '180s');
  assert.equal(formatSecs(300), '5m');
  assert.equal(formatSecs(600), '10m');
  assert.equal(formatSecs(3600), '1h');
  assert.equal(formatSecs(5400), '90m');
  assert.equal(formatSecs(1.5), '1,5s');
  assert.equal(formatSecs(0), '0s');
});
