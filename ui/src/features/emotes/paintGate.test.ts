import assert from 'node:assert/strict';
import { test } from 'node:test';

import { paintClassOf } from './paintGate.ts';

test('V109-F78 a paint draws no class while its rules load, then gets it', () => {
  const ready = new Set<string>();
  assert.equal(paintClassOf('P1', ready), null);
  ready.add('P1');
  assert.equal(paintClassOf('P1', ready), 'paint-P1');
  assert.equal(paintClassOf('P2', ready), null);
  assert.equal(paintClassOf(null, ready), null);
});
