import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
import { matches, preview, sections, since, uptimeShort } from './lists.ts';

function card(login: string, viewers: number | null, startedAt = '2026-10-10T10:00:00Z', lastLive: string | null = null): ChannelCard {
  return {
    id: login,
    login,
    name: login.toUpperCase(),
    avatar: null,
    stream:
      viewers === null
        ? null
        : { title: 't', viewers, startedAt, rerun: false, game: { id: '1', slug: 'chess', name: 'Chess' }, tags: [] },
    lastLive: lastLive ?? (viewers === null ? null : startedAt),
    lastGame: null,
    lastTitle: null,
  };
}

test('sections sort live and offline and put favorites first', () => {
  const follows = [
    card('a', 10, '2026-10-10T12:00:00Z'),
    card('b', 50, '2026-10-10T08:00:00Z'),
    card('c', null, '', '2026-10-01T00:00:00Z'),
    card('d', null, '', '2026-10-09T00:00:00Z'),
    card('e', null),
  ];
  const fav = [card('c', null, '', '2026-10-01T00:00:00Z'), card('z', 5)];
  const s = sections(follows, fav, { live: 'viewers', offline: 'lastLive' });
  assert.deepEqual(s.favorites.map((c) => c.id), ['z', 'c']);
  assert.deepEqual(s.live.map((c) => c.id), ['b', 'a']);
  assert.deepEqual(s.offline.map((c) => c.id), ['d', 'e']);

  const up = sections(follows, [], { live: 'uptime', offline: 'name' });
  assert.deepEqual(up.live.map((c) => c.id), ['b', 'a']);
  assert.deepEqual(up.offline.map((c) => c.id), ['c', 'd', 'e']);
  assert.deepEqual(sections(follows, [], { live: 'name', offline: 'name' }, 'ches').live.map((c) => c.id), ['a', 'b']);
});

test('filter matches login, name and category', () => {
  assert.ok(matches(card('xqc', 1), 'XQ'));
  assert.ok(matches(card('xqc', 1), 'chess'));
  assert.ok(!matches(card('xqc', null), 'chess'));
  assert.ok(matches(card('xqc', null), '  '));
});

test('since reads like a person would say it', () => {
  const now = new Date(2026, 9, 10, 15).getTime();
  assert.equal(since(new Date(2026, 9, 10, 1).toISOString(), now), 'today');
  assert.equal(since(new Date(2026, 9, 9, 23).toISOString(), now), 'yesterday');
  assert.equal(since(new Date(2026, 9, 7, 12).toISOString(), now), '3 days ago');
  assert.equal(since(new Date(2026, 8, 20).toISOString(), now), '2 weeks ago');
  assert.equal(since(new Date(2026, 4, 1).toISOString(), now), '5 months ago');
  assert.equal(since(new Date(2023, 9, 1).toISOString(), now), '3 years ago');
  // 60 days back: never "1 months ago".
  assert.equal(since(new Date(2026, 7, 11, 12).toISOString(), now), '8 weeks ago');
  assert.equal(since(null, now), '');
});

test('previews refresh every five minutes', () => {
  assert.equal(preview('xqc', 440, 248, 600_000), 'https://static-cdn.jtvnw.net/previews-ttv/live_user_xqc-440x248.jpg?t=2');
  assert.equal(preview('xqc', 440, 248, 899_999), preview('xqc', 440, 248, 600_000));
});

test('uptime stops at minutes', () => {
  const start = '2026-10-10T10:00:00Z';
  const at = (s: number) => Date.parse(start) + s * 1000;
  assert.equal(uptimeShort(start, at(3118)), '51m');
  assert.equal(uptimeShort(start, at(3 * 3600 + 30 * 60 + 59)), '3h 30m');
  assert.equal(uptimeShort(start, at(31 * 3600)), '1d 7h');
  assert.equal(uptimeShort(start, at(20)), '1m');
  assert.equal(uptimeShort('x'), '');
});
