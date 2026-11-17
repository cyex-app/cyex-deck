import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { LinkPreview } from '../../ipc/bindings/LinkPreview';
import { factRows, factText, numeric } from './linkFacts.ts';

test('card facts use the app formats (V108-C25)', () => {
  assert.equal(factText({ t: 'views', n: 1_234_567 }), '1.234.567 views');
  assert.equal(factText({ t: 'viewers', n: 1 }), '1 viewer');
  assert.equal(factText({ t: 'length', secs: 7384 }), '2:03:04');
  assert.equal(factText({ t: 'date', at: '2026-10-01T12:00:00' }), '2026/10/01');
  assert.equal(factText({ t: 'date', at: 'nope' }), '');
  assert.equal(factText({ t: 'clippedBy', name: 'A' }), 'Clipped by A');
  assert.equal(factText({ t: 'likes', n: 1200 }), '1.200 likes');
  assert.equal(factText({ t: 'likes', n: 1 }), '1 like');
  assert.equal(numeric({ t: 'likes', n: 1 }), true);
  assert.equal(numeric({ t: 'game', name: 'Chess' }), false);
});

const base = { kind: 'web', title: 'T', lines: [], thumbnail: null, avatar: null, live: false, nsfw: false, finalUrl: null } as const;
type Facts = LinkPreview['facts'];

test('V111-57 a YouTube card from our resolver lists its facts like Chatterino, in our formats', () => {
  const facts: Facts = [
    { t: 'length', secs: 546 },
    { t: 'views', n: 6_234_567 },
    { t: 'date', at: '2016-09-07T12:00:00' },
    { t: 'likes', n: 148_100 },
  ];
  assert.deepEqual(factRows({ ...base, site: 'YouTube', subtitle: 'RiceLive', facts }), {
    rows: [
      { label: 'Channel', text: 'RiceLive', num: false },
      { label: 'Duration', text: '9:06', num: true },
      { label: 'Published', text: '2016/09/07', num: true },
      { label: 'Views', text: '6.234.567', num: true },
      { label: null, text: '148.100 likes', num: true },
    ],
    sub: null,
  });
  // Missing pieces are just left out; a bad date is no line.
  assert.deepEqual(factRows({ ...base, site: 'YouTube', subtitle: null, facts: [{ t: 'date', at: 'nope' }] }), { rows: [], sub: null });
  // Chatterino's cards (no facts) and Twitch cards keep their own look.
  assert.equal(factRows({ ...base, site: 'youtube.com', subtitle: null, facts: [] }), null);
  assert.equal(factRows({ ...base, kind: 'video', site: 'Twitch', subtitle: null, facts }), null);
});

test('V111-58 X, Reddit and GIF cards as rows, in each site’s words', () => {
  const x = factRows({
    ...base,
    site: 'X',
    subtitle: null,
    facts: [
      { t: 'likes', n: 325_003 },
      { t: 'date', at: '2006-03-21T20:50:14Z' },
      { t: 'comments', n: 19_261 },
      { t: 'reposts', n: 134_457 },
      { t: 'media', photos: 2, videos: 1 },
    ],
  });
  assert.deepEqual(
    x?.rows.map((r) => (r.label ? `${r.label}: ${r.text}` : r.text)),
    ['Published: 2006/03/21', '325.003 likes', '134.457 reposts', '19.261 replies', 'Media: 2 photos, 1 video'],
  );
  const reddit = factRows({
    ...base,
    site: 'Reddit',
    subtitle: 'r/LivestreamFail · u/someuser',
    facts: [
      { t: 'likes', n: 4321 },
      { t: 'comments', n: 1 },
      { t: 'community', name: 'r/LivestreamFail' },
      { t: 'author', name: 'u/someuser' },
      { t: 'spoiler' },
    ],
  });
  assert.deepEqual(
    reddit?.rows.map((r) => (r.label ? `${r.label}: ${r.text}` : r.text)),
    ['Posted by: u/someuser', 'Subreddit: r/LivestreamFail', '4.321 upvotes', '1 comment', 'Spoiler'],
  );
  assert.equal(reddit?.sub, null, 'the rows already say who and where');
  // A subtitle no row repeats stays under the title.
  assert.equal(factRows({ ...base, site: 'Imgur', subtitle: 'Gallery', facts: [{ t: 'views', n: 5 }] })?.sub, 'Gallery');
  const tenor = factRows({ ...base, site: 'Tenor', subtitle: null, facts: [{ t: 'author', name: 'Akt66' }] });
  assert.deepEqual(tenor?.rows, [{ label: 'Posted by', text: 'Akt66', num: false }]);
  assert.equal(factText({ t: 'media', photos: 0, videos: 3 }), '3 videos');
  assert.equal(numeric({ t: 'spoiler' }), false);
});
