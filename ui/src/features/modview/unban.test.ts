import assert from 'node:assert/strict';
import { test } from 'node:test';

import { bannedBy, isSplit, messageCount, nextAfter, statusText } from './unban.ts';

test('V108-C36 side by side only popped out and at least 480 px wide', () => {
  assert.equal(isSplit(true, 480), true);
  assert.equal(isSplit(true, 479), false);
  assert.equal(isSplit(false, 1200), false);
  assert.equal(isSplit(true, 0), false);
});

test('V108-C36 after a request is resolved the next one opens, else the one above', () => {
  assert.equal(nextAfter(['a', 'b', 'c'], 'a'), 'b');
  assert.equal(nextAfter(['a', 'b', 'c'], 'c'), 'b');
  assert.equal(nextAfter(['a'], 'a'), null);
  assert.equal(nextAfter([], 'a'), null);
  assert.equal(nextAfter(['x', 'y'], 'gone'), 'x');
});

test('V108-C36 statuses read as words', () => {
  assert.equal(statusText('APPROVED'), 'Approved');
  assert.equal(statusText('denied'), 'Denied');
  assert.equal(statusText('ACKNOWLEDGED'), 'Closed');
  assert.equal(statusText('CANCELED'), 'Withdrawn');
  assert.equal(statusText('NEW_ONE'), 'New_one');
  assert.equal(statusText(''), '');
});

test('V108-C36 message counts: dots, and Twitch stops at 1.000', () => {
  assert.equal(messageCount(0), '0');
  assert.equal(messageCount(999), '999');
  assert.equal(messageCount(1000), '1.000+');
});

test('V108-C36 who banned them, or hidden', () => {
  assert.equal(bannedBy('SomeMod', false), 'SomeMod');
  assert.equal(bannedBy('SomeMod', true), 'Hidden');
  assert.equal(bannedBy(null, false), 'Unknown');
  assert.equal(bannedBy('  ', false), 'Unknown');
});
