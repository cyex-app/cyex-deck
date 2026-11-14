import { test } from 'node:test';
import assert from 'node:assert/strict';
import { FailedSet, next } from './avatarFallback.ts';

test('loaded in time stays 7TV', () => {
  assert.equal(next('trying', 'load'), 'loaded');
  assert.equal(next('loaded', 'timeout'), 'loaded');
});

test('an error falls back to Twitch', () => {
  assert.equal(next('trying', 'error'), 'failed');
});

test('a timeout falls back to Twitch', () => {
  assert.equal(next('trying', 'timeout'), 'failed');
});

test('a late load after the fallback stays Twitch', () => {
  assert.equal(next('failed', 'load'), 'failed');
});

test('a cached failure is remembered and bounded', () => {
  const f = new FailedSet(2);
  f.add('a');
  f.add('b');
  assert.equal(f.has('a'), true);
  f.add('c');
  assert.equal(f.has('a'), false);
  assert.equal(f.has('c'), true);
  assert.equal(f.size, 2);
});
