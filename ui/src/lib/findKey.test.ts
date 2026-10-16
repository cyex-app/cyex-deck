import assert from 'node:assert/strict';
import { test } from 'node:test';

import { blockBrowserFind, isFindKey, stoppedByUs } from './findKey.ts';

function press(key: string, mods: Partial<Record<'ctrlKey' | 'metaKey' | 'altKey' | 'shiftKey', boolean>> = {}) {
  const e = {
    key,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    defaultPrevented: false,
    preventDefault() {
      e.defaultPrevented = true;
    },
    ...mods,
  };
  return e;
}

test('plain Ctrl+F is the find key, in either case', () => {
  assert.ok(isFindKey(press('f', { ctrlKey: true })));
  assert.ok(isFindKey(press('F', { ctrlKey: true })));
});

test('other combos are not', () => {
  assert.ok(!isFindKey(press('f')));
  assert.ok(!isFindKey(press('f', { ctrlKey: true, shiftKey: true })));
  assert.ok(!isFindKey(press('f', { ctrlKey: true, altKey: true })));
  assert.ok(!isFindKey(press('g', { ctrlKey: true })));
});

test('Ctrl+F is stopped and marked; other keys are left alone', () => {
  const find = press('f', { ctrlKey: true });
  blockBrowserFind(find);
  assert.ok(find.defaultPrevented);
  assert.ok(stoppedByUs(find));

  const other = press('k', { ctrlKey: true });
  blockBrowserFind(other);
  assert.ok(!other.defaultPrevented);
  assert.ok(!stoppedByUs(other));
});

test('a key someone else already used is not claimed', () => {
  const e = press('f', { ctrlKey: true });
  e.defaultPrevented = true;
  blockBrowserFind(e);
  assert.ok(!stoppedByUs(e));
});
