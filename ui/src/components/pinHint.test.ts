import assert from 'node:assert/strict';
import { test } from 'node:test';

import { PIN_LABEL, pinHint } from './pinHint.ts';

test('V110-A29 the pin keeps one label, the hint says the state', () => {
  assert.equal(PIN_LABEL, 'Pin on top');
  assert.match(pinHint(true), /^Pinned\./);
  assert.match(pinHint(false), /^Keeps/);
});
