import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { MOD_CYCLE_MS, MOVING, moves, wallStart } from './phase.ts';

test('the start time follows the wall clock', () => {
  assert.equal(wallStart(1_700_000_012_345), -(1_700_000_012_345 % 6000));
});

test('every modifier period divides the cycle, and every moving one has a rule', () => {
  const css = readFileSync(new URL('../../design/emotes.css', import.meta.url), 'utf8');
  const found: string[] = [];
  for (const m of css.matchAll(/\.m-(\w+)\s*\{\s*animation:\s*emote-\w+\s+([\d.]+)s([^;]*);/g)) {
    const [, name, secs, rest] = m;
    const period = Number(secs) * 1000 * (rest.includes('alternate') ? 2 : 1);
    assert.ok((MOVING as readonly string[]).includes(name), `${name} isn't listed as moving`);
    assert.equal(MOD_CYCLE_MS % Math.round(period), 0, `${name}: ${period} ms doesn't divide the cycle`);
    found.push(name);
  }
  for (const name of MOVING) assert.ok(found.includes(name), `${name} has no rule`);
});

test('which classes move', () => {
  assert.equal(moves('m-flipX m-rotate'), true);
  assert.equal(moves('m-flipX'), false);
  assert.equal(moves(''), false);
});
