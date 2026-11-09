import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { BadgeView } from '../../ipc/bindings/BadgeView';
import { endDate, minutesLeft, percent, progress, statusLine, wanted } from './badges.ts';

const badge = (over: Partial<BadgeView> = {}): BadgeView => ({
  campaignId: 'c1',
  name: 'Foo',
  image: '',
  game: 'Game',
  minutesRequired: 60,
  minutesWatched: 34,
  endsAt: 0,
  picked: true,
  allowedChannels: 0,
  helperChannels: 0,
  ...over,
});
const ch = (name: string, over: { mining?: boolean; watching?: boolean } = {}) => ({
  name,
  mining: false,
  watching: false,
  ...over,
});

test('progress is minutes of minutes, capped and grouped', () => {
  assert.equal(progress(badge()), '34/60 min');
  assert.equal(progress(badge({ minutesWatched: 99, minutesRequired: 60 })), '60/60 min');
  assert.equal(progress(badge({ minutesWatched: 0, minutesRequired: 1500 })), '0/1.500 min');
});

test('minutes left and percent never leave their range', () => {
  assert.equal(minutesLeft(badge()), 26);
  assert.equal(minutesLeft(badge({ minutesWatched: 80 })), 0);
  assert.equal(percent(badge({ minutesRequired: 0 })), 0);
  assert.equal(percent(badge({ minutesWatched: 90 })), 100);
});

test('end dates are yyyy/mm/dd, empty when unknown', () => {
  assert.equal(endDate(0), '');
  assert.match(endDate(1_800_000_000), /^\d{4}\/\d{2}\/\d{2}$/);
});

test('earning line names the badge, channel and progress', () => {
  const line = statusLine({ badges: [badge()], focus: 'c1', channels: [ch('X', { mining: true })] });
  assert.equal(line, 'Earning Foo badge on X · 34/60 min');
});

test('waits when nothing live can earn it, with the retry time if known', () => {
  const input = { badges: [badge()], focus: null, channels: [ch('X')] };
  assert.equal(statusLine(input), 'Badge waits: nothing live can earn it');
  assert.equal(statusLine({ ...input, retry: '14:30' }), 'Badge waits: nothing live can earn it, looking again 14:30');
});

test('paused when two streams play here', () => {
  const channels = [ch('A', { watching: true }), ch('B', { watching: true })];
  assert.equal(statusLine({ badges: [badge()], focus: null, channels }), 'Badge paused: 2 streams playing here.');
});

test('no line without a wanted badge', () => {
  assert.equal(statusLine({ badges: undefined, focus: null, channels: [] }), '');
  assert.equal(statusLine({ badges: [badge({ picked: false })], focus: null, channels: [] }), '');
  assert.deepEqual(wanted([badge({ minutesWatched: 60 })]), []);
});
