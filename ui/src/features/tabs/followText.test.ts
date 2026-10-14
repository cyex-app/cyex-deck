import assert from 'node:assert/strict';
import { test } from 'node:test';

import { followToast } from './followText.ts';

test('success says what happened', () => {
  assert.deepEqual(followToast(true, 'Foo'), { kind: 'info', text: 'Followed Foo' });
  assert.deepEqual(followToast(false, 'Foo'), { kind: 'info', text: 'Unfollowed Foo' });
});

test('failure gives the reason', () => {
  assert.deepEqual(followToast(true, 'Foo', 'You follow too many channels.'), {
    kind: 'error',
    text: "Couldn't follow: You follow too many channels.",
  });
  assert.deepEqual(followToast(false, 'Foo', new Error('x')), { kind: 'error', text: "Couldn't unfollow: x" });
});

test('failure without a reason names the channel', () => {
  assert.equal(followToast(true, 'Foo', null).text, "Couldn't follow Foo.");
  assert.equal(followToast(true, 'Foo', '  ').text, "Couldn't follow Foo.");
});
