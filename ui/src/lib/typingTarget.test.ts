import assert from 'node:assert/strict';
import { test } from 'node:test';

import { typingTarget } from './typingTarget.ts';

test('V110-F83 fields and editable boxes are typing, the rest is not', () => {
  assert.equal(typingTarget({ tagName: 'INPUT' }), true);
  assert.equal(typingTarget({ tagName: 'TEXTAREA' }), true);
  assert.equal(typingTarget({ tagName: 'SELECT' }), true);
  assert.equal(typingTarget({ tagName: 'DIV', isContentEditable: true }), true);
  assert.equal(typingTarget({ tagName: 'DIV' }), false);
  assert.equal(typingTarget({ tagName: 'BUTTON' }), false);
  assert.equal(typingTarget(null), false);
});
