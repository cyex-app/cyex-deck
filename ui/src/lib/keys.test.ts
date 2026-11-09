import assert from 'node:assert/strict';
import { test } from 'node:test';
import { uniqueKeys } from './keys.ts';

test('no repeats: keys unchanged', () => {
  assert.deepEqual(uniqueKeys(['a', 'b', 'c'], (s) => s), ['a', 'b', 'c']);
});

test('repeats get ~2, ~3', () => {
  assert.deepEqual(uniqueKeys(['a', 'b', 'a', 'a'], (s) => s), ['a', 'b', 'a~2', 'a~3']);
});

test('a real key that looks like a suffix stays unique', () => {
  const k = uniqueKeys(['a', 'a~2', 'a'], (s) => s);
  assert.equal(new Set(k).size, 3);
});

test('empty list', () => {
  assert.deepEqual(uniqueKeys([], (s: string) => s), []);
});

test('V110-F81 an IME confirm is composing, by flag or by keyCode 229', async () => {
  const { composing, enterKey } = await import('./keys.ts');
  assert.equal(composing({ isComposing: true }), true);
  assert.equal(composing({ keyCode: 229 }), true);
  assert.equal(composing({ isComposing: false, keyCode: 13 }), false);
  assert.equal(composing({}), false);
  assert.equal(enterKey({ key: 'Enter', isComposing: true }), false);
  assert.equal(enterKey({ key: 'Enter', keyCode: 229 }), false);
  assert.equal(enterKey({ key: 'Enter', keyCode: 13 }), true);
  assert.equal(enterKey({ key: 'a' }), false);
});
