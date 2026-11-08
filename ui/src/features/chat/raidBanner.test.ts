import assert from 'node:assert/strict';
import { test } from 'node:test';
import { handFold, joinShows, newPin, PIN_OPEN, raidFold, raidLeft, raidSlim, viewersText } from './raidBanner.ts';

test('join shows only by setting, signed in, once per raid', () => {
  assert.equal(joinShows(false, 'r1', '900', null), false, 'off by default');
  assert.equal(joinShows(true, 'r1', '900', null), true);
  assert.equal(joinShows(true, null, '900', null), false, 'no raid');
  assert.equal(joinShows(true, 'r1', null, null), false, 'signed out');
  assert.equal(joinShows(true, 'r1', '900', 'r1'), false, 'joined');
  assert.equal(joinShows(true, 'r2', '900', 'r1'), true, 'a new raid');
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
