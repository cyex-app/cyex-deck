import assert from 'node:assert/strict';
import { test } from 'node:test';

import { lineStacks, stacks } from './stacked.ts';

test('V107-Mj zalgo stacks, real writing does not', () => {
  const zalgo = 'Z' + '̖̗́̀̋̏';
  assert.equal(stacks(zalgo), true);
  // Vietnamese and Devanagari, decomposed: two or three marks.
  assert.equal(stacks('Việt'), false);
  assert.equal(stacks('क्षिं'), false);
  assert.equal(stacks('Tiếng Việt, ภาษาไทย, हिन्दी'), false);
  assert.equal(stacks(null), false);
});

test('V107-Mj a line stacks when any text in it does', () => {
  assert.equal(lineStacks([{ t: 'text', text: 'hi ' }, { t: 'mention', text: '@a' + '́'.repeat(5), login: 'a' }]), true);
  assert.equal(lineStacks([{ t: 'text', text: 'hi' }, { t: 'break' }]), false);
  assert.equal(lineStacks([], 'sub' + '̖'.repeat(9)), true);
});
