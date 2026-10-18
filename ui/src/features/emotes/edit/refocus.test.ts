import assert from 'node:assert/strict';
import { test } from 'node:test';

import { refocus } from './refocus.ts';

function el() {
  const e = { focused: 0, focus: () => void e.focused++ };
  return e;
}

test('focus goes back to the opener', () => {
  const btn = el();
  const home = el();
  let asked = '';
  const root = { querySelector: (s: string) => ((asked = s), btn) };
  assert.equal(refocus(root, 'rename', home), btn);
  assert.equal(asked, '[data-opener="rename"] button:not(:disabled)');
  assert.equal(btn.focused, 1);
  assert.equal(home.focused, 0);
});

test('opener gone or disabled: the heading', () => {
  const home = el();
  assert.equal(refocus({ querySelector: () => null }, 'add', home), home);
  assert.equal(home.focused, 1);
});

test('no opener known: the heading', () => {
  const home = el();
  assert.equal(refocus({ querySelector: () => el() }, null, home), home);
});

test('nothing to focus', () => {
  assert.equal(refocus(undefined, 'add', undefined), null);
});
