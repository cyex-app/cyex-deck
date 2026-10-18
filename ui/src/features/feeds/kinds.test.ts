import assert from 'node:assert/strict';
import { test } from 'node:test';

import { FEED_CHOICES, feedSpec, KIND, sameFeed } from './kinds.ts';

test('U8 a choice becomes the tab it makes', () => {
  assert.deepEqual(feedSpec('mentions', 'x'), { kind: 'rules', name: 'Mentions' });
  assert.deepEqual(feedSpec('custom', '  Subs '), { kind: 'rules', name: 'Subs' });
  assert.deepEqual(feedSpec('live', 'x'), { kind: 'live', name: '' });
  assert.deepEqual(feedSpec('automod', ''), { kind: 'automod', name: '' });
});

test('U8 every choice but the rules feeds has a kind entry', () => {
  for (const c of FEED_CHOICES) {
    const k = feedSpec(c.value, 'a').kind;
    if (k !== 'rules') assert.ok(KIND[k].empty);
  }
});

test('U8 sameFeed compares custom names case-insensitively', () => {
  const tile = { feed: ' Subs ', feedKind: 'rules' as const };
  assert.ok(sameFeed(tile, feedSpec('custom', 'subs')));
  assert.ok(!sameFeed(tile, feedSpec('custom', 'other')));
  assert.ok(!sameFeed(tile, feedSpec('custom', '')));
  assert.ok(!sameFeed(tile, feedSpec('mentions', '')));
});

test('U8 sameFeed matches mentions and own kinds', () => {
  assert.ok(sameFeed({ feed: 'mentions', feedKind: 'rules' }, feedSpec('mentions', '')));
  assert.ok(sameFeed({ feed: 'Live', feedKind: 'live' }, feedSpec('live', '')));
  assert.ok(!sameFeed({ feed: 'Live', feedKind: 'live' }, feedSpec('automod', '')));
  assert.ok(!sameFeed({ feed: null, feedKind: 'live' }, feedSpec('live', '')));
  assert.ok(!sameFeed({ feed: 'x', feedKind: 'live' }, feedSpec('custom', 'x')));
});
