import assert from 'node:assert/strict';
import { test } from 'node:test';

import { pickShown } from './pickPlayer.ts';

const on = { visible: () => true };
const off = { visible: () => false };
const on2 = { visible: () => true };

test('V110-F83 the last used player wins only while it is shown', () => {
  assert.equal(pickShown(on, [off, on, on2]), on);
  assert.equal(pickShown(off, [off, on2]), on2);
});

test('V110-F83 a background tab\'s player is never picked', () => {
  assert.equal(pickShown(off, [off]), null);
  assert.equal(pickShown(null, []), null);
  assert.equal(pickShown({}, [{}]), null);
});
