import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { EmoteCard } from '../../ipc/bindings/EmoteCard';
import { addedDate, cardChips, cardFacts, emotePage, expectedFacts, foldChips, opensTip } from './cardFacts.ts';

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
    card({
      channel: 'Foo',
      channelLogin: 'foo',
      owner: 'Art',
      addedBy: 'mod',
      addedAt: '2026-10-18T12:00:00',
      setUrl: 'https://7tv.app/emote-sets/S',
    }),
    'twitch',
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
  );
  assert.deepEqual(f[0], { label: 'Channel', value: 'Foo', link: 'channel', login: 'foo', avatar: 'https://a/p.png' });
  assert.deepEqual(f[1], { label: 'Artist', value: 'Art', link: 'channel', login: 'art' });
});

test('Twitch globals show no channel, only the artist', () => {
  const f = cardFacts(
    card({ flags: ['global'], channel: 'Twitch', channelLogin: 'twitch', owner: 'Art', artistLogin: 'art' }),
    'twitch',
  );
  assert.deepEqual(f, [{ label: 'Artist', value: 'Art', link: 'channel', login: 'art' }]);
  assert.deepEqual(cardFacts(card({ flags: ['global'], channel: 'Twitch', owner: 'Art' }), 'twitch'), [
    { label: 'Artist', value: 'Art' },
  ]);
  // Sub, follower and bits emotes keep their channel, opened in the app.
  for (const flag of ['tier1', 'follower', 'bits'] as const) {
    const [c] = cardFacts(card({ flags: ['channel', flag], channel: 'Foo', channelLogin: 'foo' }), 'twitch');
    assert.deepEqual(c, { label: 'Channel', value: 'Foo', link: 'channel', login: 'foo' });
  }
});

test('channel without a login is plain text', () => {
  assert.deepEqual(cardFacts(card({ channel: 'Foo' }), 'twitch'), [{ label: 'Channel', value: 'Foo' }]);
});

test('owners link to their page with the host as tip', () => {
  const [s] = cardFacts(card({ owner: 'ayy', ownerUrl: 'https://7tv.app/users/01X' }), 'seventv');
  assert.deepEqual(s, { label: 'By', value: 'ayy', link: 'web', url: 'https://7tv.app/users/01X', tip: 'Opens 7tv.app' });
  const [b] = cardFacts(card({ owner: 'x', ownerUrl: 'https://betterttv.com/users/abc' }), 'bttv');
  assert.equal(b.link === 'web' && b.tip, 'Opens betterttv.com');
  const [f] = cardFacts(card({ owner: 'x', ownerUrl: 'https://www.frankerfacez.com/emoticons/user/x' }), 'ffz');
  assert.equal(f.link === 'web' && f.tip, 'Opens frankerfacez.com');
});

test('a broken owner url stays plain', () => {
  const [f] = cardFacts(card({ owner: 'ayy', ownerUrl: 'nope' }), 'seventv');
  assert.equal(f.link, undefined);
  assert.equal(opensTip(null), null);
});

test('7TV set is named by its scope', () => {
  const set = (flags: EmoteCard['flags']) =>
    cardFacts(card({ provider: 'seventv', flags, setUrl: 'https://7tv.app/emote-sets/S' }), 'seventv')[0];
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

test('FFZ date reads as uploaded, with the local date and time as tip (kb-2)', () => {
  // No zone: read as local time, like the tip shows it.
  const [f] = cardFacts(card({ addedAt: '2020-01-02T09:05:00' }), 'ffz');
  assert.deepEqual(f, { label: 'Uploaded', value: '2020/01/02', tip: '2020/01/02 09:05' });
  const utc = addedDate('2020-01-02T21:30:00Z');
  const d = new Date('2020-01-02T21:30:00Z');
  const p = (n: number) => String(n).padStart(2, '0');
  assert.equal(utc?.tip, `${d.getFullYear()}/${p(d.getMonth() + 1)}/${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`);
  assert.equal(addedDate('nope'), null);
  assert.equal(addedDate(null), null);
});

test('chips: scope, then warnings (the state), then the rest (kb-3)', () => {
  assert.deepEqual(cardChips(null, false), []);
  assert.deepEqual(
    cardChips(null, true).map((c) => c.text),
    ['Gigantified'],
  );
  const c = cardChips(card({ flags: ['channel', 'tier2', 'animated'], state: 'Archived' }), true);
  assert.deepEqual(
    c.map((x) => x.text),
    ['Channel', 'Archived', 'Tier 2', 'Animated', 'Gigantified'],
  );
  assert.equal(c[0].tip, 'Belongs to a channel');
  assert.equal(c[2].tip, 'For subscribers at tier 2 and up');
  assert.deepEqual(c[1], { text: 'Archived', tip: 'Twitch lists it as archived', warn: true, keep: true });
  assert.ok(c.every((x) => x.tip));
});

test('7TV chips: overlaying, private, warnings, rename tip', () => {
  const c = cardChips(
    card({
      provider: 'seventv',
      original: 'TriSad',
      flags: ['personal', 'overlaying', 'private', 'unlisted', 'nsfw', 'aliased'],
    }),
    false,
  );
  assert.deepEqual(
    c.map((x) => x.text),
    ['Personal', 'Private', 'Unlisted', 'NSFW', 'Overlaying', 'Aliased'],
  );
  assert.match(c[0].tip, /personal set/);
  assert.match(c[4].tip, /zero-width/);
  assert.equal(c[1].keep, true);
  assert.equal(c[1].warn, undefined);
  assert.deepEqual(
    c.filter((x) => x.warn).map((x) => x.text),
    ['Unlisted', 'NSFW'],
  );
  assert.equal(c[5].tip, 'Aliased in this channel');
  // Scope comes first even when the flags list it later.
  assert.deepEqual(
    cardChips(card({ flags: ['animated', 'global'] }), false).map((x) => x.text),
    ['Global', 'Animated'],
  );
});

test('Shared sits with the scope, before the warnings (fa-3)', () => {
  assert.deepEqual(
    cardChips(card({ provider: 'bttv', flags: ['channel', 'nsfw', 'animated', 'shared'] }), false).map((x) => x.text),
    ['Channel', 'Shared', 'NSFW', 'Animated'],
  );
});

test('lines held while loading, by provider and kind (fb-4)', () => {
  assert.equal(expectedFacts('seventv', 'reckH'), 4);
  assert.equal(expectedFacts('bttv', 'cvMask'), 1);
  assert.equal(expectedFacts('ffz', 'Hmm'), 2);
  // Twitch: a channel prefix means a sub, follower or bits emote; else a global.
  for (const n of ['lirikSHUCKS', 'sodaPlead', 'lirik10K', 'elis0']) assert.equal(expectedFacts('twitch', n), 2, n);
  for (const n of ['LUL', 'Kreygasm', 'PogChamp', 'BooHoo', ':)']) assert.equal(expectedFacts('twitch', n), 0, n);
});

test('bits chip counts when known', () => {
  assert.deepEqual(cardChips(card({ flags: ['bits'], bits: 1000 }), false)[0], {
    text: '1.000 bits',
    tip: 'Unlocked by cheering 1.000 bits in the channel',
  });
  assert.equal(cardChips(card({ flags: ['bits'] }), false)[0].text, 'Bits');
});

test('7TV and BTTV owners without a name still link their profile (ct-11, ka-1)', () => {
  const [f] = cardFacts(card({ ownerUrl: 'https://7tv.app/users/01X' }), 'seventv');
  assert.deepEqual(f, {
    label: 'By',
    value: '7TV profile',
    link: 'web',
    url: 'https://7tv.app/users/01X',
    tip: 'Opens 7tv.app',
  });
  assert.deepEqual(cardFacts(card({ ownerUrl: 'https://betterttv.com/users/a' }), 'bttv'), [
    { label: 'By', value: 'BTTV profile', link: 'web', url: 'https://betterttv.com/users/a', tip: 'Opens betterttv.com' },
  ]);
  assert.deepEqual(cardFacts(card({ ownerUrl: 'https://x/u' }), 'ffz'), []);
});

const chips = (...texts: string[]) => texts.map((text) => ({ text, tip: text }));

test('chips that fit stay unfolded', () => {
  const c = chips('a', 'b', 'c', 'd');
  assert.deepEqual(foldChips(c, '7TV'), { shown: c, rest: [] });
});

test('no fixed cap: everything that fits in two rows shows (kb-3)', () => {
  const c = chips('a', 'b', 'c', 'd', 'e', 'f', 'g', 'h');
  assert.deepEqual(foldChips(c, '7TV'), { shown: c, rest: [] });
});

test('folding goes by the measured width (kb-3)', () => {
  const c = chips('a', 'b', 'c', 'd', 'e');
  // Each 100 px wide in a 250 px row: two per line, the provider's included.
  const f = foldChips(c, '7TV', 250, () => 100);
  assert.deepEqual(f.shown, c.slice(0, 2));
  assert.deepEqual(f.rest, c.slice(2));
});

test('warning chips never fold (kb-3)', () => {
  const warn = { text: 'NSFW', tip: 'x', warn: true, keep: true };
  const c = [...chips('Channel'), warn, ...chips('Long chip one', 'Long chip two', 'Long chip three', 'Long chip four')];
  const f = foldChips(c, '7TV', 200);
  assert.ok(f.shown.includes(warn));
  assert.ok(f.rest.length >= 2);
  assert.ok(f.rest.every((x) => !x.keep));
  // Even with no room at all, the warnings stay.
  const tight = foldChips(c, '7TV', 10);
  assert.deepEqual(tight.shown, [warn]);
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
  const c = chips('Archived set', 'Limited time', 'Overlaying', 'Unlisted', 'Aliased', 'Two-factor');
  assert.ok(foldChips(c, '7TV').rest.length >= 2);
  // A wide row takes them all.
  assert.deepEqual(foldChips(c, '7TV', 1000).rest, []);
});

test('emote pages without the details, as the app builds them (ka-6)', () => {
  assert.equal(emotePage('seventv', '01E', 'x'), 'https://7tv.app/emotes/01E');
  assert.equal(emotePage('bttv', 'abc', 'x'), 'https://betterttv.com/emotes/abc');
  assert.equal(emotePage('twitch', 'emotesv2_ab', 'x'), 'https://chatvau.lt/emote/twitch/emotesv2_ab');
  assert.equal(emotePage('ffz', '720507', 'monkaHmm'), 'https://www.frankerfacez.com/emoticon/720507-monkaHmm');
  assert.equal(emotePage('ffz', '1', 'a/b c'), 'https://www.frankerfacez.com/emoticon/1-a%2Fb%20c');
});
