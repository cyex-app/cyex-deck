import assert from 'node:assert/strict';
import { test } from 'node:test';

import { InFlight, mergePatch } from './mergePatch.ts';

test('objects merge, arrays and values replace', () => {
  const v = { a: 1, b: { c: 2, d: [1, 2] }, e: 'x' };
  assert.deepEqual(mergePatch(v, { b: { d: [3] } }), { a: 1, b: { c: 2, d: [3] }, e: 'x' });
  assert.deepEqual(mergePatch(v, { a: 5, f: { g: true } }), { a: 5, b: { c: 2, d: [1, 2] }, e: 'x', f: { g: true } });
  assert.deepEqual(v, { a: 1, b: { c: 2, d: [1, 2] }, e: 'x' }, 'the old value is left alone');
});

test('null keeps the value until Rust answers with the default', () => {
  assert.deepEqual(mergePatch({ a: 1, b: 2 }, { a: null, b: 3 }), { a: 1, b: 3 });
});

test('a second list edit builds on the first', () => {
  const rule = { pattern: '', sound: false };
  let list = mergePatch({ list: [rule] }, { list: [{ ...rule, pattern: 'lol' }] });
  list = mergePatch(list, { list: [{ ...list.list[0], sound: true }] });
  assert.deepEqual(list.list, [{ pattern: 'lol', sound: true }]);
});

test('only the newest edit keeps its answer', () => {
  const f = new InFlight<'mentions' | 'chat'>();
  const a = f.start('mentions');
  const b = f.start('mentions');
  const c = f.start('chat');
  assert.equal(f.busy('mentions'), true);
  assert.equal(f.settle('mentions', a), false, 'an older answer is dropped');
  assert.equal(f.settle('mentions', b), true);
  assert.equal(f.busy('mentions'), false);
  assert.equal(f.settle('chat', c), true);
});
