import assert from 'node:assert/strict';
import { test } from 'node:test';

import { docked, type InTile, opened, step } from './inTab.ts';

const route = (login: string) => `modview/${login}`;

test('room: show; none: pop out; none right after coming back: close', () => {
  assert.equal(step(false, false), 'show');
  assert.equal(step(false, true), 'show');
  assert.equal(step(true, false), 'popout');
  assert.equal(step(true, true), 'close');
});

test('a popped-out mod view comes back when its window closes', () => {
  const out: InTile = { ...opened('xqc'), out: true };
  let tiles = new Map([['t1', out], ['t2', opened('lirik')]]);

  // Before its window shows up nothing happens, so an unrelated change can't bring it back.
  assert.equal(docked(tiles, ['settings'], route), null);

  tiles = docked(tiles, ['modview/xqc'], route)!;
  assert.deepEqual(tiles.get('t1'), { ...out, seen: true });
  assert.equal(docked(tiles, ['modview/xqc', 'settings'], route), null, 'still open');

  tiles = docked(tiles, ['settings'], route)!;
  assert.deepEqual(tiles.get('t1'), { login: 'xqc', out: false, seen: false, back: true });
  assert.deepEqual(tiles.get('t2'), opened('lirik'), 'others untouched');
});
