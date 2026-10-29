import assert from 'node:assert/strict';
import { test } from 'node:test';

import { suggestKey } from './suggestKeys.ts';

test('V108-N09 row: Tab and arrows move and wrap, Enter picks, Esc closes', () => {
  assert.deepEqual(suggestKey('row', { key: 'Tab' }, 0, 3), { do: 'move', index: 1 });
  assert.deepEqual(suggestKey('row', { key: 'Tab' }, 2, 3), { do: 'move', index: 0 });
  assert.deepEqual(suggestKey('row', { key: 'Tab', shift: true }, 0, 3), { do: 'move', index: 2 });
  assert.deepEqual(suggestKey('row', { key: 'ArrowRight' }, 1, 3), { do: 'move', index: 2 });
  assert.deepEqual(suggestKey('row', { key: 'ArrowLeft' }, 0, 3), { do: 'move', index: 2 });
  assert.deepEqual(suggestKey('row', { key: 'Enter' }, 1, 3), { do: 'pick' });
  assert.deepEqual(suggestKey('row', { key: 'Escape' }, 1, 3), { do: 'close' });
});

test('V108-N09 row: up and down belong to the box', () => {
  assert.equal(suggestKey('row', { key: 'ArrowDown' }, 0, 3), null);
  assert.equal(suggestKey('row', { key: 'ArrowUp' }, 0, 3), null);
});

test('V108-N09 list keeps its keys: Up/Down move, Tab and Enter pick', () => {
  assert.deepEqual(suggestKey('list', { key: 'ArrowDown' }, 2, 3), { do: 'move', index: 0 });
  assert.deepEqual(suggestKey('list', { key: 'ArrowUp' }, 0, 3), { do: 'move', index: 2 });
  assert.deepEqual(suggestKey('list', { key: 'Tab' }, 0, 3), { do: 'pick' });
  assert.deepEqual(suggestKey('list', { key: 'Enter' }, 0, 3), { do: 'pick' });
  assert.equal(suggestKey('list', { key: 'ArrowLeft' }, 0, 3), null);
});

test('Enter sends for command and argument suggestions; Ctrl+Enter is never a pick', () => {
  assert.equal(suggestKey('list', { key: 'Enter' }, 0, 3, true), null);
  assert.equal(suggestKey('list', { key: 'Enter', ctrl: true }, 0, 3), null);
  assert.equal(suggestKey('row', { key: 'Enter' }, 0, 3, true), null);
  assert.deepEqual(suggestKey('list', { key: 'Tab' }, 0, 3, true), { do: 'pick' });
});

test('nothing to pick from: no suggestion keys', () => {
  assert.equal(suggestKey('row', { key: 'Tab' }, 0, 0), null);
});
