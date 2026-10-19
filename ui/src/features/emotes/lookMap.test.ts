import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Look } from '../../ipc/bindings/Look.ts';
import { applyAnswer, applyPushed, trim } from './lookMap.ts';

const look = { paint: 'p' } as unknown as Look;

test('a push skips users the UI never asked for', () => {
  const m = new Map<string, Look | null>([['a', null]]);
  applyPushed(m, { a: look, b: look });
  assert.equal(m.get('a'), look);
  assert.equal(m.has('b'), false);
});

test('an answer keeps all and trims the oldest past the cap', () => {
  const m = new Map<string, Look | null>();
  applyAnswer(m, { a: look, b: look, c: look }, 2);
  assert.deepEqual([...m.keys()], ['b', 'c']);
  trim(m, 1);
  assert.deepEqual([...m.keys()], ['c']);
});
