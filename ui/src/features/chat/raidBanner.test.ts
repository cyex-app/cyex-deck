import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  cancelShows,
  handFold,
  newPin,
  ownChannel,
  PIN_OPEN,
  raidAbout,
  raidBar,
  raidFold,
  raidFrac,
  raidLeft,
  raidSlim,
  raidTabsToClose,
  tickShows,
  viewersText,
} from './raidBanner.ts';

test('the Join tick always shows in another channel while signed in, no setting', () => {
  assert.equal(tickShows('r1', '900'), true);
  assert.equal(tickShows(null, '900'), false, 'no raid');
  assert.equal(tickShows('r1', null), false, 'signed out');
  assert.equal(tickShows('r1', '900', true), false, 'your own channel');
});

test('countdown reads like a clock', () => {
  const t = 1000_000;
  assert.equal(raidLeft(1090, t), '1:30');
  assert.equal(raidLeft(1009, t), '0:09');
  assert.equal(raidLeft(1000.2, t), '0:01');
  assert.equal(raidLeft(1000, t), 'now');
  assert.equal(raidLeft(900, t), 'now');
});

test('viewers are grouped and counted', () => {
  const f = (n: number) => String(n).replace(/\B(?=(\d{3})+$)/g, '.');
  assert.equal(viewersText(1, f), '1 viewer');
  assert.equal(viewersText(1234, f), '1.234 viewers');
});

test('an open pin folds for the raid and opens again', () => {
  const during = raidFold(PIN_OPEN, true);
  assert.deepEqual(during, { open: false, before: true, hand: false });
  assert.equal(raidSlim(during), true);
  assert.equal(raidFold(during, true), during, 'updates change nothing');
  assert.deepEqual(raidFold(during, false), PIN_OPEN);
});

test('a folded pin stays folded after the raid', () => {
  const folded = { open: false, before: null, hand: false };
  const during = raidFold(folded, true);
  assert.deepEqual(raidFold(during, false), folded);
});

test('a fold by hand during the raid is kept', () => {
  const opened = handFold(raidFold(PIN_OPEN, true));
  assert.deepEqual(opened, { open: true, before: true, hand: true });
  assert.equal(raidSlim(opened), false);
  const refolded = handFold(opened);
  assert.equal(raidFold(refolded, false).open, false, 'folded by hand stays folded');
  assert.deepEqual(handFold(PIN_OPEN), { open: false, before: null, hand: false }, 'no raid: a plain fold');
});

test('a new pin during a raid starts folded', () => {
  assert.deepEqual(newPin(false), PIN_OPEN);
  assert.deepEqual(raidFold(newPin(true), false), PIN_OPEN);
});

test('in your own channel there is no Join tick, only Cancel for the manager account', () => {
  assert.equal(ownChannel('900', '900'), true);
  assert.equal(ownChannel('900', '7'), false);
  assert.equal(ownChannel(null, '900'), false);
  assert.equal(tickShows('r1', '900', ownChannel('900', '900')), false);
  assert.equal(cancelShows('r1', true, '900', '900'), true);
  assert.equal(cancelShows('r1', false, '900', '900'), false, 'not your channel');
  assert.equal(cancelShows('r1', true, '55', '900'), false, 'the manager runs another channel');
  assert.equal(cancelShows('r1', true, null, '900'), false, 'no manager');
  assert.equal(cancelShows(null, true, '900', '900'), false, 'no raid');
});

test('the bar drains from the remaining share over the remaining time', () => {
  const t = 1000_000; // ms; unix 1000 s
  assert.deepEqual(raidBar(1000, 1090, t), { from: 1, ms: 90_000, delay: 0 });
  assert.deepEqual(raidBar(950, 1090, t), { from: 90 / 140, ms: 90_000, delay: 0 });
  assert.deepEqual(raidBar(910, 1090, t), { from: 0.5, ms: 90_000, delay: 0 });
  assert.deepEqual(raidBar(1000, 1090, t + 45_000), { from: 0.5, ms: 45_000, delay: 0 });
  assert.deepEqual(raidBar(900, 1000, t), { from: 0, ms: 0, delay: 0 }, 'time is up');
  assert.equal(raidBar(1000, 1090, t - 5000).delay, 5000, 'not started yet');
  assert.equal(raidBar(1000, 1090, t - 5000).from, 1);
  assert.equal(raidBar(1000, 1090, t - 200_000).from, 1, 'never above full');
  assert.equal(raidBar(1090, 1090, t).from, 1, 'no span: the usual 90 s');
});

test('reduced motion shows the level for the clock', () => {
  const t = 1000_000;
  assert.equal(raidFrac(1000, 1090, t), 1);
  assert.equal(raidFrac(1000, 1090, t + 45_000), 0.5);
  assert.equal(raidFrac(1000, 1090, t + 90_000), 0);
  assert.equal(raidFrac(1000, 1090, t + 200_000), 0);
});

test('the second line keeps what it has', () => {
  assert.equal(raidAbout('Speedrun night', 'Celeste'), 'Speedrun night · Celeste');
  assert.equal(raidAbout('', 'Celeste'), 'Celeste');
  assert.equal(raidAbout('  ', null), '');
  assert.equal(raidAbout('Just chatting', undefined), 'Just chatting');
});

test('a raid pop-out that closed without docking takes its tab with it', () => {
  const seen = new Set<string>();
  assert.deepEqual(raidTabsToClose(['t1'], new Set(), seen), [], 'not shown yet: kept');
  assert.deepEqual(raidTabsToClose(['t1'], new Set(['t1']), seen), [], 'shown');
  assert.deepEqual(raidTabsToClose(['t1'], new Set(['t1']), seen), []);
  assert.deepEqual(raidTabsToClose(['t1'], new Set(), seen), ['t1'], 'window closed: the tab goes');
  assert.equal(seen.has('t1'), false);
});

test('a docked raid pop-out keeps its tab', () => {
  const seen = new Set<string>();
  raidTabsToClose(['t1'], new Set(['t1']), seen);
  // Dock back drops the mark before the window closes.
  assert.deepEqual(raidTabsToClose([], new Set(), seen), []);
  assert.equal(seen.size, 0);
});
