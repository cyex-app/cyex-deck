import assert from 'node:assert/strict';
import { test } from 'node:test';

import { focusTipAllowed, tipAllowed } from './tooltipGate.ts';

test('tipAllowed stays shut during a drag (rb-1)', () => {
  assert.equal(tipAllowed({ label: 'Drag to move' }, true), false);
  assert.equal(tipAllowed({ label: 'Drag to move' }, false), true);
});

test('tipAllowed needs something to say and its when', () => {
  assert.equal(tipAllowed({ label: '' }, false), false);
  assert.equal(tipAllowed({ label: '', image: 'x.png' }, false), true);
  assert.equal(tipAllowed({ label: 'a', when: () => false }, false), false);
});

test('V108-F59i focus after Escape, Enter or Space shows no tip; Tab and a pointer do', () => {
  assert.equal(focusTipAllowed('Escape'), false);
  assert.equal(focusTipAllowed('Enter'), false);
  assert.equal(focusTipAllowed(' '), false);
  assert.equal(focusTipAllowed('Tab'), true);
  assert.equal(focusTipAllowed('ArrowRight'), true);
  assert.equal(focusTipAllowed(null), true);
});
