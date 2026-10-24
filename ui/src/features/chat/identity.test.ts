import assert from 'node:assert/strict';
import { test } from 'node:test';

import { gridMove, hexColor, namedColor } from './identity.ts';

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

test('V108-ID grid keys step, jump and find the cell above or below', () => {
  // Row 1: a wide None (middle 20) and two cells; row 2: three cells.
  const cells = [
    { x: 20, y: 10 },
    { x: 60, y: 10 },
    { x: 100, y: 10 },
    { x: 10, y: 50 },
    { x: 50, y: 50 },
    { x: 90, y: 50 },
  ];
  assert.equal(gridMove('ArrowRight', 0, cells), 1);
  assert.equal(gridMove('ArrowRight', 5, cells), 5);
  assert.equal(gridMove('ArrowLeft', 0, cells), 0);
  assert.equal(gridMove('Home', 4, cells), 0);
  assert.equal(gridMove('End', 1, cells), 5);
  assert.equal(gridMove('ArrowDown', 1, cells), 4);
  assert.equal(gridMove('ArrowDown', 2, cells), 5);
  assert.equal(gridMove('ArrowUp', 3, cells), 0);
  assert.equal(gridMove('ArrowUp', 0, cells), 0);
  assert.equal(gridMove('ArrowDown', 5, cells), 5);
  assert.equal(gridMove('Enter', 0, cells), null);
  assert.equal(gridMove('ArrowDown', 0, []), null);
});
