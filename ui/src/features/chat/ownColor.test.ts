import assert from 'node:assert/strict';
import { test } from 'node:test';

import { contrast, ownNameColor } from './ownColor.ts';

const rgb = (h: string): [number, number, number] => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number];

test('empty or broken colours leave their own', () => {
  assert.equal(ownNameColor(''), null);
  assert.equal(ownNameColor(undefined), null);
  assert.equal(ownNameColor('#12345'), null);
  assert.equal(ownNameColor('red'), null);
});

test('a readable colour stays as it is', () => {
  const c = ownNameColor('#ff7f50')!;
  assert.equal(c.dark, '#ff7f50');
});

test('dark and light variants read on their pages', () => {
  for (const pick of ['#000000', '#ffffff', '#0000ff', '#ffff00', '#145aff']) {
    const c = ownNameColor(pick)!;
    assert.ok(contrast(rgb(c.dark), [0x1e, 0x1f, 0x23]) >= 4.5, `dark ${pick}`);
    assert.ok(contrast(rgb(c.dark), [0x26, 0x27, 0x2c]) >= 4.5, `dark surface ${pick}`);
    assert.ok(contrast(rgb(c.light), [0xf3, 0xf3, 0xf4]) >= 4.5, `light ${pick}`);
    assert.ok(contrast(rgb(c.light), [0xff, 0xff, 0xff]) >= 4.5, `light surface ${pick}`);
  }
});
