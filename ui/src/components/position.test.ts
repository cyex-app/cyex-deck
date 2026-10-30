import assert from 'node:assert/strict';
import { test } from 'node:test';
import { blocks, CONTENT, type Side } from './position.ts';

test('block and rest tile the content area for every side and kind', () => {
  for (const kind of ['chat', 'tabs'] as const) {
    for (const side of ['left', 'right', 'top', 'bottom'] as Side[]) {
      const { block, rest } = blocks(side, kind);
      assert.equal(block.w * block.h + rest.w * rest.h, CONTENT.w * CONTENT.h, `${kind} ${side}`);
      for (const r of [block, rest]) {
        assert.ok(r.x >= CONTENT.x && r.y >= CONTENT.y, `${kind} ${side} starts inside`);
        assert.ok(r.x + r.w <= CONTENT.x + CONTENT.w && r.y + r.h <= CONTENT.y + CONTENT.h, `${kind} ${side} ends inside`);
      }
    }
  }
});

test('the block hugs the side it names', () => {
  assert.equal(blocks('left', 'chat').block.x, CONTENT.x);
  assert.equal(blocks('top', 'tabs').block.y, CONTENT.y);
  const r = blocks('right', 'chat').block;
  assert.equal(r.x + r.w, CONTENT.x + CONTENT.w);
  const b = blocks('bottom', 'tabs').block;
  assert.equal(b.y + b.h, CONTENT.y + CONTENT.h);
});
