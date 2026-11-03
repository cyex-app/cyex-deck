import assert from 'node:assert/strict';
import { test } from 'node:test';

import { banLine, bannedBy, commentCount, isSplit, listMove, messageCount, nextAfter, orderRequests, statusText } from './unban.ts';

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

test('V108-C36b requests newest first, or oldest; unreadable times last either way', () => {
  const rows = [
    { id: 'a', createdAt: '2026-11-01T10:00:00Z' },
    { id: 'x', createdAt: '' },
    { id: 'b', createdAt: '2026-11-02T10:00:00Z' },
  ];
  assert.deepEqual(orderRequests(rows, true).map((r) => r.id), ['b', 'a', 'x']);
  assert.deepEqual(orderRequests(rows, false).map((r) => r.id), ['a', 'b', 'x']);
  assert.deepEqual(orderRequests([], true), []);
});

test('V108-C36b arrow keys move through the list and stop at its ends', () => {
  assert.equal(listMove('ArrowDown', 0, 3), 1);
  assert.equal(listMove('ArrowDown', 2, 3), 2);
  assert.equal(listMove('ArrowUp', 0, 3), 0);
  assert.equal(listMove('Home', 2, 3), 0);
  assert.equal(listMove('End', 0, 3), 2);
  assert.equal(listMove('Enter', 0, 3), null);
  assert.equal(listMove('ArrowDown', 0, 0), null);
});

test('V108-C36b mod comments count: own, then shared as +n', () => {
  assert.equal(commentCount([]), '0');
  assert.equal(commentCount([{ shared: false }, { shared: false }]), '2');
  assert.equal(commentCount([{ shared: false }, { shared: true }]), '1 +1');
  assert.equal(commentCount([{ shared: true }]), '0 +1');
});

test('V108-C36b the ban line names the mod unless hidden or unknown', () => {
  assert.equal(banLine({ by: 'Kuni', until: null }, 'lumen_fox', false), 'Kuni banned lumen_fox');
  assert.equal(banLine({ by: 'Kuni', until: '2026-11-04T00:00:00Z' }, 'lumen_fox', false), 'Kuni timed out lumen_fox');
  assert.equal(banLine({ by: 'Kuni', until: null }, 'lumen_fox', true), 'lumen_fox was banned');
  assert.equal(banLine({ by: ' ', until: '2026-11-04T00:00:00Z' }, 'lumen_fox', false), 'lumen_fox was timed out');
});
