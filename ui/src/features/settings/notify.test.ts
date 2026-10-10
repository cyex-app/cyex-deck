import assert from 'node:assert/strict';
import { test } from 'node:test';

import { group } from './notify.ts';

test("Twitch's switches group like twitch.tv", () => {
  const groups = group([
    { category: 'LIVE', platform: 'email', on: false },
    { category: 'LIVE', platform: 'onsite', on: true },
    { category: 'LIVE', platform: 'smart', on: true },
    { category: 'ALL', platform: 'email', on: true },
    { category: 'following_category', platform: 'email', on: true },
    { category: 'CHAT', platform: 'push', on: true },
    { category: 'NEW_THING', platform: 'push', on: false },
  ]);
  assert.deepEqual(
    groups.map((g) => [g.label, g.categories.map((c) => c.label)]),
    [
      ['Channels you follow', ['Going live']],
      ['People', ['Chat mentions']],
      ['Other', ['New thing']],
    ],
  );
  assert.deepEqual(groups[0].categories[0].on, { email: false, onsite: true });
});
