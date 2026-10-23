import assert from 'node:assert/strict';
import { test } from 'node:test';

import { cheerTotal, cheerWords, isCommand, setCheer } from './cheer.ts';

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
