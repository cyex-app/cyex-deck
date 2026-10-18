import assert from 'node:assert/strict';
import { test } from 'node:test';

import { tipAllowed } from './tooltipGate.ts';

test('tipAllowed stays shut during a drag (rb-1)', () => {
  assert.equal(tipAllowed({ label: 'Drag to move' }, true), false);
  assert.equal(tipAllowed({ label: 'Drag to move' }, false), true);
});

test('tipAllowed needs something to say and its when', () => {
  assert.equal(tipAllowed({ label: '' }, false), false);
  assert.equal(tipAllowed({ label: '', image: 'x.png' }, false), true);
  assert.equal(tipAllowed({ label: 'a', when: () => false }, false), false);
});
