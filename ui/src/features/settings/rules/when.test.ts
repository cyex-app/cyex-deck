import assert from 'node:assert/strict';
import { test } from 'node:test';
import { cleanWhen, NO_WHEN, splitCategories, splitChannels, whenSummary } from './when.ts';

test('V110-C59 no summary while nothing is set', () => {
  assert.equal(whenSummary(null), null);
  assert.equal(whenSummary(NO_WHEN), null);
  assert.equal(cleanWhen({ ...NO_WHEN, notChannels: true }), null);
});

test('V110-C59 sums up what is set', () => {
  assert.equal(whenSummary({ ...NO_WHEN, channels: ['a', 'b', 'c'], live: 'live' }), 'Only in 3 channels · live');
  assert.equal(whenSummary({ ...NO_WHEN, channels: ['xqc'], notChannels: true }), 'Not in xqc');
  assert.equal(whenSummary({ ...NO_WHEN, role: 'mod', burst: 3, burstSecs: 10 }), 'As mod · 3 in 10 s');
  assert.equal(whenSummary({ ...NO_WHEN, categories: ['Just Chatting'], firstOnly: true }), 'Just Chatting · first message');
});

test('V110-C59 splits lists', () => {
  assert.deepEqual(splitChannels('#Foo, bar  foo'), ['foo', 'bar']);
  assert.deepEqual(splitCategories('Just Chatting, Minecraft,'), ['Just Chatting', 'Minecraft']);
});
