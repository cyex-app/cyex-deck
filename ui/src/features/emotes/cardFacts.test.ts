import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { EmoteCard } from '../../ipc/bindings/EmoteCard';
import { cardChips, cardFacts } from './cardFacts.ts';

function card(over: Partial<EmoteCard> = {}): EmoteCard {
  return {
    provider: 'twitch',
    id: '1',
    name: 'Kappa',
    original: null,
    kind: null,
    channel: null,
    owner: null,
    addedBy: null,
    addedAt: null,
    zeroWidth: false,
    unlisted: false,
    animated: false,
    page: null,
    channelLogin: null,
    ownerUrl: null,
    state: null,
    ...over,
  };
}

test('facts keep their order: channel, artist, added by, date', () => {
  const f = cardFacts(card({ channel: 'Foo', channelLogin: 'foo', owner: 'Art', addedBy: 'mod' }), 'twitch', '2026/10/18');
  assert.deepEqual(
    f.map((x) => x.label),
    ['Channel', 'Artist', 'Added by', 'Added'],
  );
  assert.deepEqual(f[0], { label: 'Channel', value: 'Foo', link: 'channel', login: 'foo' });
});

test('channel without a login is plain text', () => {
  assert.deepEqual(cardFacts(card({ channel: 'Foo' }), 'twitch', null), [{ label: 'Channel', value: 'Foo' }]);
});

test('7TV owner links to its page with the host as tip', () => {
  const [f] = cardFacts(card({ owner: 'ayy', ownerUrl: 'https://7tv.app/users/01X' }), 'seventv', null);
  assert.deepEqual(f, { label: 'By', value: 'ayy', link: 'web', url: 'https://7tv.app/users/01X', tip: 'Opens 7tv.app' });
});

test('a broken owner url stays plain', () => {
  const [f] = cardFacts(card({ owner: 'ayy', ownerUrl: 'nope' }), 'seventv', null);
  assert.equal(f.link, undefined);
});

test('FFZ date reads as uploaded, with the full date as tip', () => {
  const [f] = cardFacts(card({ addedAt: '2020-01-02T00:00:00Z' }), 'ffz', '2020/01/02');
  assert.deepEqual(f, { label: 'Uploaded', value: '2020/01/02', tip: '2020-01-02T00:00:00Z' });
});

test('chips: state warns, gigantified only when flagged', () => {
  assert.deepEqual(cardChips(null, false), []);
  assert.deepEqual(cardChips(null, true), [{ text: 'Gigantified' }]);
  assert.deepEqual(cardChips(card({ kind: 'Tier 1', state: 'Archived', unlisted: true }), false), [
    { text: 'Tier 1' },
    { text: 'Archived', warn: true },
    { text: 'Unlisted', warn: true },
  ]);
});
