import assert from 'node:assert/strict';
import { test } from 'node:test';

import { clicked, splitDock, stillOpen } from './dock.ts';

test('V107-R29 panels with an icon get their own button, the rest go in the chip', () => {
  const s = splitDock([{ id: 'a', icon: 'users' }, { id: 'chat' }, { id: 'b', icon: 'zap' }]);
  assert.deepEqual(s.icons.map((i) => i.id), ['a', 'b']);
  assert.deepEqual(s.rest.map((i) => i.id), ['chat']);
});

test('V107-R29 a click opens that menu, again closes it, another switches', () => {
  assert.equal(clicked(null, 'a', false), 'a');
  assert.equal(clicked('a', 'a', false), null);
  assert.equal(clicked('a', 'b', false), 'b');
});

test('V107-R29 a drag never opens the menu', () => {
  assert.equal(clicked(null, 'a', true), null);
  assert.equal(clicked('b', 'a', true), null);
});

test('V107-R29 the menu closes once its panel left the dock', () => {
  assert.equal(stillOpen('a', ['a', 'b']), 'a');
  assert.equal(stillOpen('a', ['b']), null);
  assert.equal(stillOpen(null, ['a']), null);
});
