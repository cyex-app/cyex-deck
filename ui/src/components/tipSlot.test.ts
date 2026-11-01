import assert from 'node:assert/strict';
import { test } from 'node:test';

import { tipSlot } from './tipSlot.ts';

test('an answer that arrives before the tip shows waits for it (V108-C25)', () => {
  const s = tipSlot<string, string>();
  s.claim('a', 'loading');
  assert.equal(s.put('a', 'card'), true);
  // The tip shows after its delay and reads what came in meanwhile.
  assert.equal(s.read('a'), 'card');
});

test('a stale answer never lands on another link', () => {
  const s = tipSlot<string, string>();
  s.claim('a', 'loading');
  s.claim('b', 'loading');
  assert.equal(s.put('a', 'card for a'), false);
  assert.equal(s.read('b'), 'loading');
  assert.equal(s.read('a'), undefined);
});

test('after the pointer leaves, nothing lands', () => {
  const s = tipSlot<string, string>();
  s.claim('a', 'loading');
  s.release();
  assert.equal(s.put('a', 'card'), false);
  assert.equal(s.owner(), null);
  s.claim('a');
  assert.equal(s.read('a'), undefined);
});

test('hovering the same link again keeps its card', () => {
  const s = tipSlot<string, string>();
  s.claim('a', 'loading');
  s.put('a', 'card');
  s.claim('a', 'loading');
  assert.equal(s.read('a'), 'card');
});
