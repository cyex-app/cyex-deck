import assert from 'node:assert/strict';
import { test } from 'node:test';

import { countOn, group, sends, smart } from './notify.ts';

test('smart delivery is the LIVE switch on the smart platform (T5)', () => {
  assert.equal(smart([{ category: 'LIVE', platform: 'smart', on: true }]), true);
  assert.equal(smart([{ category: 'ALL', platform: 'smart', on: true }, { category: 'LIVE', platform: 'smart', on: false }]), false);
  assert.equal(smart([{ category: 'LIVE', platform: 'email', on: true }]), null);
});

test("Twitch's switches group like twitch.tv", () => {
  const s = group([
    { category: 'LIVE', platform: 'email', on: false },
    { category: 'LIVE', platform: 'onsite', on: true },
    { category: 'LIVE', platform: 'smart', on: true },
    { category: 'ALL', platform: 'email', on: true },
    { category: 'following_category', platform: 'email', on: true },
    { category: 'CHAT', platform: 'push', on: true },
    { category: 'NEW_THING', platform: 'push', on: false },
    { category: 'new_category', platform: 'push', on: true },
  ]);
  assert.deepEqual(
    s.groups.map((g) => [g.label, g.master?.id ?? null, g.categories.map((c) => c.label)]),
    [
      ['Channels you follow', 'following_category', ['Going live']],
      ['People', null, ['Chat mentions']],
      ['Other', null, ['New thing']],
    ],
  );
  assert.deepEqual(s.groups[0].categories[0].on, { email: false, onsite: true });
  assert.deepEqual(s.all?.on, { email: true });
});

test('V101-06 a group sends only while it and Everything are on', () => {
  const s = group([
    { category: 'ALL', platform: 'email', on: false },
    { category: 'ALL', platform: 'push', on: true },
    { category: 'social_category', platform: 'push', on: false },
    { category: 'social_category', platform: 'onsite', on: true },
    { category: 'CHAT', platform: 'onsite', on: true },
    { category: 'CLIPS', platform: 'onsite', on: false },
  ]);
  const people = s.groups[0];
  assert.equal(sends(s, people, 'email'), false);
  assert.equal(sends(s, people, 'push'), false);
  assert.equal(sends(s, people, 'onsite'), true);
  assert.deepEqual(countOn(people, 'onsite'), [1, 2]);
  assert.deepEqual(countOn(people, 'email'), [0, 0]);
});
