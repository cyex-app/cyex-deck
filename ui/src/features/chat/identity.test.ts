import assert from 'node:assert/strict';
import { test } from 'node:test';

import { hexColor, namedColor } from './identity.ts';

test('V107-Mk free colors are #RRGGBB', () => {
  assert.equal(hexColor('#1e90ff'), '#1E90FF');
  assert.equal(hexColor(' 1E90FF '), '#1E90FF');
  assert.equal(hexColor('#1E90F'), null);
  assert.equal(hexColor('blue'), null);
});

test('V107-Mk a preset is known by name', () => {
  assert.equal(namedColor('#1e90ff'), 'Dodger blue');
  assert.equal(namedColor('#123456'), null);
  assert.equal(namedColor(null), null);
});
