import assert from 'node:assert/strict';
import { test } from 'node:test';

import { InFlight, mergePatch, sameJson } from './mergePatch.ts';

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
  assert.equal(f.settle('mentions', a), null, 'an older answer is dropped');
  assert.deepEqual(f.settle('mentions', b), {});
  assert.equal(f.busy('mentions'), false);
  assert.deepEqual(f.settle('chat', c), {});
});

test('a change skipped while busy is reported with the newest answer', () => {
  const f = new InFlight<'tabs'>();
  const a = f.start('tabs');
  f.miss('tabs', { v: 1 });
  assert.deepEqual(f.settle('tabs', a), { missed: { value: { v: 1 } } });
  const b = f.start('tabs');
  assert.deepEqual(f.settle('tabs', b), {}, 'cleared once reported');
});

test('no miss, nothing reported', () => {
  const f = new InFlight<'tabs'>();
  const a = f.start('tabs');
  assert.deepEqual(f.settle('tabs', a), {});
});

test('a miss is reported on the newest edit only', () => {
  const f = new InFlight<'tabs'>();
  const a = f.start('tabs');
  const b = f.start('tabs');
  f.miss('tabs', 'x');
  assert.equal(f.settle('tabs', a), null);
  assert.deepEqual(f.settle('tabs', b), { missed: { value: 'x' } });
});

test('a miss while idle is not kept', () => {
  const f = new InFlight<'tabs'>();
  f.miss('tabs', 'x');
  const a = f.start('tabs');
  assert.deepEqual(f.settle('tabs', a), {});
});

test('same JSON ignores key order', () => {
  assert.equal(sameJson({ a: 1, b: [1, { c: 2, d: 3 }] }, { b: [1, { d: 3, c: 2 }], a: 1 }), true);
  assert.equal(sameJson({ a: 1 }, { a: 1, b: 2 }), false);
  assert.equal(sameJson([1, 2], [2, 1]), false);
  assert.equal(sameJson({ a: null }, { a: 0 }), false);
});
