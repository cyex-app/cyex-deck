import assert from 'node:assert/strict';
import { test } from 'node:test';

import { inScrollbar } from './scrollZone.ts';

test('the scrollbar zone and the wheel reach beside it (V110-U51)', () => {
  const box = { right: 300, bar: 8, scrolls: true };
  assert.equal(inScrollbar(box, 296), true, 'on the bar');
  assert.equal(inScrollbar(box, 290), false, 'left of the bar, no reach');
  assert.equal(inScrollbar(box, 290, 8), true, 'within the wheel reach');
  assert.equal(inScrollbar(box, 280, 8), false, 'past the reach');
  assert.equal(inScrollbar(box, 301, 8), false, 'right of the bar');
  assert.equal(inScrollbar({ ...box, scrolls: false }, 296, 8), false, 'nothing to scroll');
  assert.equal(inScrollbar({ ...box, bar: 0 }, 300, 8), false, 'no bar drawn');
});
