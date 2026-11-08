import assert from 'node:assert/strict';
import { test } from 'node:test';

import { actionFor, defaultTimeout, triggerOf } from './nameClick.ts';

const ev = (o: Partial<Parameters<typeof triggerOf>[0]> = {}) => ({ button: 0, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, ...o });

test('each click kind picks its binding', () => {
  assert.equal(triggerOf(ev()), 'plain');
  assert.equal(triggerOf(ev({ button: 1 })), 'middle');
  assert.equal(triggerOf(ev({ ctrlKey: true })), 'ctrl');
  assert.equal(triggerOf(ev({ shiftKey: true })), 'shift');
  assert.equal(triggerOf(ev({ ctrlKey: true, shiftKey: true })), 'ctrl');
});

test('Alt, Meta and the right button are not name clicks', () => {
  assert.equal(triggerOf(ev({ altKey: true })), null);
  assert.equal(triggerOf(ev({ metaKey: true })), null);
  assert.equal(triggerOf(ev({ button: 2 })), null);
});

test('what cannot happen here opens the usercard', () => {
  const none = { reply: false, mention: false, mod: false };
  for (const a of ['reply', 'mention', 'timeout', 'ban'] as const) assert.equal(actionFor(a, none), 'usercard');
  const all = { reply: true, mention: true, mod: true };
  for (const a of ['reply', 'mention', 'timeout', 'ban'] as const) assert.equal(actionFor(a, all), a);
});

test('copy, logs, none and the usercard need nothing', () => {
  const none = { reply: false, mention: false, mod: false };
  for (const a of ['copy', 'logs', 'none', 'usercard'] as const) assert.equal(actionFor(a, none), a);
});

test('the default timeout is the shortest quick length', () => {
  assert.equal(defaultTimeout([600, 60, 3600]), 60);
  assert.equal(defaultTimeout([]), 600);
});
