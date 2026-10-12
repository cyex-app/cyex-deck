import assert from 'node:assert/strict';
import { test } from 'node:test';

import { isMentionsFeed, whenText } from './mentionsTab.ts';

test('the Mentions tab is the feed named Mentions, in any case (V104-01)', () => {
  assert.ok(isMentionsFeed(' mentions '));
  assert.ok(isMentionsFeed('Mentions'));
  assert.ok(!isMentionsFeed('Pings'));
  assert.ok(!isMentionsFeed(null));
});

test('feed lines add the date before today, the time when lines show none (V104-02)', () => {
  const now = new Date(2026, 9, 12, 18, 0).getTime();
  const today = new Date(2026, 9, 12, 9, 5).getTime();
  const before = new Date(2026, 9, 11, 23, 59).getTime();
  assert.equal(whenText(today, false, now), '09:05');
  assert.equal(whenText(today, true, now), '');
  assert.equal(whenText(before, false, now), '2026/10/11 23:59');
  assert.equal(whenText(before, true, now), '2026/10/11');
});
