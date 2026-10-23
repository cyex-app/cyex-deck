import assert from 'node:assert/strict';
import { test } from 'node:test';

import { canAfford, cheerFloor, cheerPresets, cheerTotal, cheerWords, isCommand, setCheer, startAmount, wordMin } from './cheer.ts';

const known = (p: string) => ['cheer', '4head', 'showlove'].includes(p);

test('cheer words follow Twitch’s rule (V107-Bn)', () => {
  assert.deepEqual(cheerWords('Cheer100 gg 4Head5 Kappa10 a1 Cheer100x showlove1000', known), [
    { prefix: 'cheer', amount: 100 },
    { prefix: '4head', amount: 5 },
    { prefix: 'showlove', amount: 1000 },
  ]);
});

test('commands are never cheers, /me is (V107-Bn)', () => {
  assert.equal(isCommand('/ban x'), true);
  assert.equal(isCommand('/me Cheer1'), false);
  assert.equal(isCommand('/me\tCheer1'), false);
  assert.equal(isCommand('/meCheer1'), true);
  assert.equal(cheerTotal('/w x Cheer100', known), 0);
  assert.equal(cheerTotal('/me Cheer100 Cheer1 hype', known), 101);
  assert.equal(cheerTotal('hello', known), 0);
});

test('the picker sets or adds the cheer word (V107-Bn)', () => {
  assert.equal(setCheer('', 'Cheer', 100), 'Cheer100 ');
  assert.equal(setCheer('gg', 'Cheer', 100), 'gg Cheer100 ');
  assert.equal(setCheer('gg cheer5 wp', 'Cheer', 500), 'gg Cheer500 wp');
  assert.equal(setCheer('ShowLove10 gg', 'Cheer', 1), 'ShowLove10 gg Cheer1 ');
});

test('cheer lookalikes are plain words, like Rust (V107-H1)', () => {
  const cheer = (p: string) => p === 'cheer';
  for (const t of ['cheers100', 'Cheer100Kappa', 'Cheer１００', 'Chéer100', 'Cheer100!', 'Cheer​100', 'xCheer100']) {
    assert.equal(cheerTotal(t, cheer), 0, t);
  }
  assert.equal(cheerTotal('gg Cheer5', cheer), 5);
  assert.equal(cheerTotal('Cheer0100', cheer), 100);
});

test('picker presets follow the minimum and the balance (V107-H10a)', () => {
  const open = { cheerMin: 1, emoteMin: 1, balance: null };
  assert.deepEqual(cheerPresets(open).map((p) => p.bits), [1, 100, 500, 1000, 5000, 10000]);
  assert.ok(cheerPresets(open).every((p) => p.ok));
  const rich = { cheerMin: 1, emoteMin: 1, balance: 700 };
  assert.deepEqual(cheerPresets(rich).map((p) => [p.bits, p.ok]), [[1, true], [100, true], [500, true], [1000, false], [5000, false], [10000, false]]);
  // The minimum leads when it isn't a preset; presets under it are gone.
  assert.deepEqual(cheerPresets({ cheerMin: 250, emoteMin: 1, balance: 5000 }).map((p) => p.bits), [250, 500, 1000, 5000, 10000]);
  assert.deepEqual(cheerPresets({ cheerMin: 1, emoteMin: 100, balance: 5000 }).map((p) => p.bits), [100, 500, 1000, 5000, 10000]);
  assert.deepEqual(cheerPresets({ cheerMin: 0, emoteMin: 0, balance: 0 }).map((p) => [p.bits, p.ok]).slice(0, 1), [[1, false]]);
  assert.deepEqual(cheerPresets({ cheerMin: 200000, emoteMin: 1, balance: null }).map((p) => p.bits), [100000]);
});

test('picker amounts: floor, word minimum, balance (V107-H10a)', () => {
  const r = { cheerMin: 300, emoteMin: 50, balance: 1000 };
  assert.equal(cheerFloor(r), 300);
  assert.equal(wordMin(r), 50);
  assert.equal(canAfford(50, r), true);
  assert.equal(canAfford(49, r), false);
  assert.equal(canAfford(1000, r), true);
  assert.equal(canAfford(1001, r), false);
  assert.equal(canAfford(1.5, r), false);
  assert.equal(canAfford(100001, { cheerMin: 1, emoteMin: 1, balance: null }), false);
  assert.equal(startAmount(100, r), 300);
  assert.equal(startAmount(500, r), 500);
});
