import assert from 'node:assert/strict';
import { test } from 'node:test';
import { BadgeMisses, RetryGate, missScope } from './badgeMiss.ts';

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

test('V110-F84 a failed read waits 60 s before the next try', () => {
  const g = new RetryGate();
  assert.equal(g.ok('room:1', 0), true);
  g.fail('room:1', 1000);
  assert.equal(g.ok('room:1', 30_000), false);
  assert.equal(g.ok('room:1', 61_000), true);
  g.fail('room:1', 61_000);
  g.done('room:1');
  assert.equal(g.ok('room:1', 62_000), true);
});
