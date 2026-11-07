import assert from 'node:assert/strict';
import { test } from 'node:test';

import { cardBadges } from './badgeLine.ts';

const sets = (l: { set: string }[]) => l.map((b) => b.set);

test('roles are dropped', () => {
  const out = cardBadges([{ set: 'broadcaster' }, { set: 'moderator' }, { set: 'lead_moderator' }, { set: 'vip' }, { set: 'staff' }, { set: 'subscriber' }]);
  assert.deepEqual(sets(out), ['subscriber']);
});
test('events first, then channel-earned, then identity; order kept within a group', () => {
  const out = cardBadges([
    { set: 'partner' }, { set: 'bits' }, { set: 'blossom-2026' }, { set: 'subscriber' }, { set: 'some-event' }, { set: 'hype-train' },
  ]);
  assert.deepEqual(sets(out), ['blossom-2026', 'some-event', 'bits', 'subscriber', 'hype-train', 'partner']);
});
test('empty stays empty', () => assert.deepEqual(cardBadges([]), []));
