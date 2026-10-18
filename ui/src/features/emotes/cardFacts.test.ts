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

test('7TV owner without a name still links its profile (ct-11)', () => {
  const [f] = cardFacts(card({ ownerUrl: 'https://7tv.app/users/01X' }), 'seventv', null);
  assert.deepEqual(f, {
    label: 'By',
    value: '7TV profile',
    link: 'web',
    url: 'https://7tv.app/users/01X',
    tip: 'Opens 7tv.app',
  });
  assert.deepEqual(cardFacts(card({ ownerUrl: 'https://betterttv.com/users/a' }), 'bttv', null), []);
});

const chips = (...texts: string[]) => texts.map((text) => ({ text, tip: text }));

test('chips that fit stay unfolded', () => {
  const c = chips('a', 'b', 'c', 'd');
  assert.deepEqual(foldChips(c, '7TV'), { shown: c, rest: [] });
});

test('the provider chip counts toward the cap (R7)', () => {
  // 6 chips + the provider's is 7: four stay, "+2" takes the sixth slot.
  const c = chips('a', 'b', 'c', 'd', 'e', 'f');
  const f = foldChips(c, '7TV');
  assert.deepEqual(f.shown, c.slice(0, 4));
  assert.deepEqual(f.rest, c.slice(4));
  assert.deepEqual(foldChips(c.slice(0, 5), '7TV'), { shown: c.slice(0, 5), rest: [] });
});

test('never folds just one chip', () => {
  for (let n = 1; n <= 12; n++) {
    const c = chips(...Array.from({ length: n }, (_, i) => `Chip ${i}`));
    const f = foldChips(c, 'Twitch');
    assert.ok(f.rest.length === 0 || f.rest.length >= 2, `n=${n}`);
    assert.equal(f.shown.length + f.rest.length, n);
  }
});

test('long chips fold to stay within two lines', () => {
  const c = chips('Personal approved', 'Personal denied', 'Overlaying', 'Unlisted', 'Renamed');
  assert.ok(foldChips(c, '7TV').rest.length >= 2);
  // A wide row takes them all.
  assert.deepEqual(foldChips(c, '7TV', 1000).rest, []);
});
