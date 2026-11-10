import assert from 'node:assert/strict';
import { test } from 'node:test';

import { SCROLLBACK_WARN, scrollbackHint } from './scrollbackHint.ts';

test('V110-A30 below the limit the hint is the plain one', () => {
  assert.equal(scrollbackHint(1000), 'Per chat. Older ones drop off the top; more uses more memory.');
  assert.equal(scrollbackHint(SCROLLBACK_WARN - 1), scrollbackHint(100));
});

test('V110-A30 from 3.000 up it warns, with a grouped number', () => {
  assert.equal(scrollbackHint(3000), '3.000 lines per chat use a lot of memory.');
  assert.equal(scrollbackHint(8000), '8.000 lines per chat use a lot of memory.');
});
