import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mergeLog } from './consoleMerge.ts';

const e = (ts: number, text: string, kind = 'log') => ({ ts, kind, text });

test('V110-F84 a live line also in the snapshot counts once', () => {
  const out = mergeLog([e(1, 'a'), e(2, 'b')], [e(2, 'b'), e(3, 'c')], 100);
  assert.deepEqual(out.map((x) => x.text), ['a', 'b', 'c']);
});

test('lines that came in meanwhile are kept, not replaced', () => {
  const out = mergeLog([e(1, 'a')], [e(5, 'new'), e(6, '> 1+1', 'in')], 100);
  assert.deepEqual(out.map((x) => x.text), ['a', 'new', '> 1+1']);
});

test('two identical live lines are both kept when the snapshot has one', () => {
  const out = mergeLog([e(1, 'x')], [e(1, 'x'), e(1, 'x')], 100);
  assert.equal(out.length, 2);
});

test('only the newest max stay', () => {
  assert.equal(mergeLog([e(1, 'a'), e(2, 'b')], [e(3, 'c')], 2)[0].text, 'b');
});
