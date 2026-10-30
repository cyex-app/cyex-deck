import assert from 'node:assert/strict';
import { test } from 'node:test';
import { BadgeMisses, missScope } from './badgeMiss.ts';

test('a missing badge asks once per catalog', () => {
  const m = new BadgeMisses();
  assert.equal(m.first('chan', 'subscriber', '3012'), true);
  assert.equal(m.first('chan', 'subscriber', '3012'), false);
  assert.equal(m.first('chan', 'subscriber', '3024'), true);
  assert.equal(m.first('other', 'subscriber', '3012'), true);
});

test('past the cap the list starts over', () => {
  const m = new BadgeMisses(2);
  assert.equal(m.first('c', 'a', '1'), true);
  assert.equal(m.first('c', 'b', '1'), true);
  assert.equal(m.first('c', 'x', '1'), true);
  assert.equal(m.first('c', 'a', '1'), true);
  assert.equal(m.first('c', 'x', '1'), false);
});

test('shared chat refs ask for their room, others for the channel', () => {
  assert.equal(missScope('chan', '123'), 'room:123');
  assert.equal(missScope('chan', undefined), 'chan');
});
