import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { EmoteCard } from '../../ipc/bindings/EmoteCard';
import { cardChips, cardFacts, foldChips, opensTip } from './cardFacts.ts';

function card(over: Partial<EmoteCard> = {}): EmoteCard {
  return {
    provider: 'twitch',
    id: '1',
    name: 'Kappa',
    original: null,
    flags: [],
    bits: null,
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
    channelAvatar: null,
    artistLogin: null,
    setUrl: null,
    ...over,
  };
}

test('facts keep their order: channel, artist, added by, date, set', () => {
  const f = cardFacts(
    card({ channel: 'Foo', channelLogin: 'foo', owner: 'Art', addedBy: 'mod', setUrl: 'https://7tv.app/emote-sets/S' }),
    'twitch',
    '2026/10/18',
  );
  assert.deepEqual(
    f.map((x) => x.label),
    ['Channel', 'Artist', 'Added by', 'Added', 'Set'],
  );
  assert.deepEqual(f[0], { label: 'Channel', value: 'Foo', link: 'channel', login: 'foo' });
});

test('channel shows its avatar; artist opens in the app', () => {
  const f = cardFacts(
    card({ channel: 'Foo', channelLogin: 'foo', channelAvatar: 'https://a/p.png', owner: 'Art', artistLogin: 'art' }),
    'twitch',
    null,
  );
  assert.deepEqual(f[0], { label: 'Channel', value: 'Foo', link: 'channel', login: 'foo', avatar: 'https://a/p.png' });
  assert.deepEqual(f[1], { label: 'Artist', value: 'Art', link: 'channel', login: 'art' });
});

test('channel without a login is plain text', () => {
  assert.deepEqual(cardFacts(card({ channel: 'Foo' }), 'twitch', null), [{ label: 'Channel', value: 'Foo' }]);
});

test('owners link to their page with the host as tip', () => {
  const [s] = cardFacts(card({ owner: 'ayy', ownerUrl: 'https://7tv.app/users/01X' }), 'seventv', null);
  assert.deepEqual(s, { label: 'By', value: 'ayy', link: 'web', url: 'https://7tv.app/users/01X', tip: 'Opens 7tv.app' });
  const [b] = cardFacts(card({ owner: 'x', ownerUrl: 'https://betterttv.com/users/abc' }), 'bttv', null);
  assert.equal(b.link === 'web' && b.tip, 'Opens betterttv.com');
  const [f] = cardFacts(card({ owner: 'x', ownerUrl: 'https://www.frankerfacez.com/emoticons/user/x' }), 'ffz', null);
  assert.equal(f.link === 'web' && f.tip, 'Opens frankerfacez.com');
});

test('a broken owner url stays plain', () => {
  const [f] = cardFacts(card({ owner: 'ayy', ownerUrl: 'nope' }), 'seventv', null);
  assert.equal(f.link, undefined);
  assert.equal(opensTip(null), null);
});

test('7TV set is named by its scope', () => {
  const set = (flags: EmoteCard['flags']) =>
    cardFacts(card({ provider: 'seventv', flags, setUrl: 'https://7tv.app/emote-sets/S' }), 'seventv', null)[0];
  assert.deepEqual(set(['channel']), {
    label: 'Set',
    value: 'Channel set',
    link: 'web',
    url: 'https://7tv.app/emote-sets/S',
    tip: 'Opens 7tv.app',
  });
  assert.equal(set(['personal']).value, 'Personal set');
  assert.equal(set(['global']).value, 'Global set');
});

test('FFZ date reads as uploaded, with the full date as tip', () => {
  const [f] = cardFacts(card({ addedAt: '2020-01-02T00:00:00Z' }), 'ffz', '2020/01/02');
  assert.deepEqual(f, { label: 'Uploaded', value: '2020/01/02', tip: '2020-01-02T00:00:00Z' });
});

test('chips follow the flags, then gigantified, then the state', () => {
  assert.deepEqual(cardChips(null, false), []);
  assert.deepEqual(
    cardChips(null, true).map((c) => c.text),
    ['Gigantified'],
  );
  const c = cardChips(card({ flags: ['channel', 'tier2', 'animated'], state: 'Archived' }), true);
  assert.deepEqual(
    c.map((x) => x.text),
    ['Channel', 'Tier 2', 'Animated', 'Gigantified', 'Archived'],
  );
  assert.equal(c[0].tip, 'Belongs to a channel');
  assert.equal(c[1].tip, 'For subscribers at tier 2 and up');
  assert.deepEqual(c[4], { text: 'Archived', tip: 'Twitch lists it as archived', warn: true });
  assert.ok(c.every((x) => x.tip));
});

test('7TV chips: overlaying, private, warnings, rename tip', () => {
  const c = cardChips(
    card({
      provider: 'seventv',
      original: 'TriSad',
      flags: ['channel', 'overlaying', 'private', 'unlisted', 'personalDenied', 'nsfw', 'aliased'],
    }),
    false,
  );
  assert.deepEqual(
    c.map((x) => x.text),
    ['Channel', 'Overlaying', 'Private', 'Unlisted', 'Personal denied', 'NSFW', 'Renamed'],
  );
  assert.equal(c[0].tip, 'In this channel’s set');
  assert.match(c[1].tip, /zero-width/);
  assert.deepEqual(
    c.filter((x) => x.warn).map((x) => x.text),
    ['Unlisted', 'NSFW'],
  );
  assert.equal(c[6].tip, 'Renamed in this channel. Its own name is TriSad');
});

test('bits chip counts when known', () => {
  assert.deepEqual(cardChips(card({ flags: ['bits'], bits: 1000 }), false)[0], {
    text: '1.000 bits',
    tip: 'Unlocked by cheering 1.000 bits in the channel',
  });
  assert.equal(cardChips(card({ flags: ['bits'] }), false)[0].text, 'Bits');
});

test('chips past the cap fold into +N', () => {
  const chips = ['a', 'b', 'c', 'd'].map((text) => ({ text, tip: text }));
  assert.equal(foldChips(chips, 4), chips);
  assert.deepEqual(foldChips(chips, 3), [chips[0], chips[1], { text: '+2', tip: 'c, d' }]);
});
