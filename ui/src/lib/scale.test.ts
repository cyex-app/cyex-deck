import assert from 'node:assert/strict';
import { test } from 'node:test';

import { pictureScale } from './scale.ts';

test('picture scale follows the screen density (V105-08)', () => {
  assert.deepEqual([1, 1.25, 1.5, 2, 2.5, 3].map(pictureScale), [1, 2, 2, 2, 4, 4]);
});
