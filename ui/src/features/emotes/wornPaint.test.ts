import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Look } from '../../ipc/bindings/Look.ts';
import { wornPaint } from './wornPaint.ts';

const both: Look = { paint: 'S1', cyextvPaint: 'ctvP1', badges: [] };
const on = (ctv: boolean, seventv: boolean) => ({ cyextvCosmetics: ctv, paints: { seventv } });

test('cyex.tv first when both switches are on', () => {
  assert.equal(wornPaint(both, on(true, true)), 'ctvP1');
});

test('each switch alone decides', () => {
  assert.equal(wornPaint(both, on(false, true)), 'S1');
  assert.equal(wornPaint(both, on(true, false)), 'ctvP1');
  assert.equal(wornPaint(both, on(false, false)), null);
});

test('a person with only one kind', () => {
  assert.equal(wornPaint({ paint: 'S1', badges: [] }, on(true, true)), 'S1');
  assert.equal(wornPaint({ paint: null, cyextvPaint: 'ctvP1', badges: [] }, on(true, true)), 'ctvP1');
  assert.equal(wornPaint(null, on(true, true)), null);
  assert.equal(wornPaint(undefined, on(true, true)), null);
});
