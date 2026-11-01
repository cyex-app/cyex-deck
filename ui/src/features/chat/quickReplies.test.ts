import assert from 'node:assert/strict';
import { test } from 'node:test';

import { menuLine } from './quickReplies.ts';

test('a short phrase stays as it is', () => {
  assert.deepEqual(menuLine('gg'), { label: 'gg', cut: false });
});

test('a long phrase is cut to 59 characters and …', () => {
  const r = menuLine('a'.repeat(80));
  assert.equal(r.cut, true);
  assert.equal(r.label, `${'a'.repeat(59)}…`);
});

test('the cut never lands inside a surrogate pair', () => {
  // 58 letters, then emoji: UTF-16 index 59 would split the first one.
  const r = menuLine('a'.repeat(58) + '😀'.repeat(10));
  assert.equal(r.label, `${'a'.repeat(58)}😀…`);
});

test('a family emoji is not split', () => {
  const fam = '👨‍👩‍👧';
  const r = menuLine('a'.repeat(58) + fam + 'b'.repeat(10));
  assert.equal(r.label, `${'a'.repeat(58)}${fam}…`);
});

test('60 characters that take more than 60 UTF-16 units are not cut', () => {
  const t = '😀'.repeat(60);
  assert.deepEqual(menuLine(t), { label: t, cut: false });
});
