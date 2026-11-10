import assert from 'node:assert/strict';
import { test } from 'node:test';

import { couldBeChannel, enterTarget, manyNames, liveOrExact, loginFromLink, matchChannels, type Candidate } from './channelMatch.ts';

const ch = (id: string, login: string, live = false, name = login): Candidate => ({
  id,
  login,
  name,
  stream: live ? { viewers: 1 } : null,
});
const ids = (list: Candidate[]) => list.map((c) => c.id);

test('an offline exact name beats a page of live hits', () => {
  const exact = [ch('1', 'sery')];
  const search = [1, 2, 3, 4, 5, 6, 7].map((n) => ch(`l${n}`, `sery${n}`, true)).concat(ch('1', 'sery'));
  const out = matchChannels(exact, search, 'sery', 6);
  assert.equal(out.length, 6);
  assert.equal(out[0].id, '1');
  assert.equal(new Set(ids(out)).size, 6);
});

test('an offline exact name stays in even when the lookup missed it', () => {
  const search = [1, 2, 3, 4, 5, 6].map((n) => ch(`l${n}`, `xsery${n}`, true)).concat(ch('1', 'Sery'));
  assert.equal(matchChannels([], search, 'sery', 6)[0].id, '1');
});

test('a link or id match is pinned on top', () => {
  const out = matchChannels([ch('71092938', 'xqc')], [ch('2', '71092938a', true)], '71092938', 6);
  assert.deepEqual(ids(out), ['71092938', '2']);
});

test('better match first, ties go live first, then the order they came in', () => {
  const search = [ch('a', 'foobar'), ch('b', 'foobaz', true), ch('c', 'my_foo', true), ch('d', 'foo_x')];
  // starts: a, b, d (b live first), then the word match c.
  assert.deepEqual(ids(matchChannels([], search, 'foo', 6)), ['b', 'a', 'd', 'c']);
});

test('the display name counts too, and nothing that matches is lost', () => {
  const search = [ch('a', 'zzz', false), ch('b', 'qq', false, 'Foo')];
  assert.deepEqual(ids(matchChannels([], search, 'foo', 6)), ['b', 'a']);
});

test('duplicates are dropped by id', () => {
  const out = matchChannels([ch('1', 'a')], [ch('1', 'a'), ch('2', 'ab'), ch('2', 'ab')], 'a', 6);
  assert.deepEqual(ids(out), ['1', '2']);
});

test('the switcher keeps live hits and the offline exact name', () => {
  const items = [ch('1', 'sery'), ch('2', 'sery2'), ch('3', 'sery3', true), ch('4', 'x', false, 'Sery')];
  assert.deepEqual(ids(liveOrExact(items, ' SERY ')), ['1', '3', '4']);
});

test('loginFromLink: channel links give the login, other text none', () => {
  assert.equal(loginFromLink('https://www.twitch.tv/rvbdev'), 'rvbdev');
  assert.equal(loginFromLink('twitch.tv/RvbDev'), 'rvbdev');
  assert.equal(loginFromLink(' http://twitch.tv/rvbdev?sr=a&foo=1#x '), 'rvbdev');
  assert.equal(loginFromLink('https://m.twitch.tv/rvbdev/about'), 'rvbdev');
  assert.equal(loginFromLink('https://www.twitch.tv/popout/rvbdev/chat?popout='), 'rvbdev');
  assert.equal(loginFromLink('https://www.twitch.tv/videos/123'), null);
  assert.equal(loginFromLink('rvbdev'), null);
  assert.equal(loginFromLink('https://example.com/rvbdev'), null);
});

test('Enter waits when the list belongs to older text (S3-01)', () => {
  const a = ch('1', 'abdu');
  assert.equal(enterTarget('zzzz', 'ab', 'looking', [a], null), 'wait');
  assert.equal(enterTarget('zzzz', 'ab', 'done', [a], null), 'wait');
  assert.equal(enterTarget('ab', 'ab', 'done', [a], null), a);
  assert.equal(enterTarget('zzzz', 'zzzz', 'done', [], null), null);
  assert.equal(enterTarget('  ', 'ab', 'done', [a], null), null);
  assert.equal(enterTarget('abdu', 'abdu', 'done', [a, ch('2', 'abdv')], a), a);
});

test('V110-F28 couldBeChannel: logins of 4 to 25 characters and twitch.tv links', () => {
  assert.equal(couldBeChannel('rvbdev'), true);
  assert.equal(couldBeChannel(' @RvbDev '), true);
  assert.equal(couldBeChannel('#rvb_dev1'), true);
  assert.equal(couldBeChannel('abc'), false);
  assert.equal(couldBeChannel('a'.repeat(26)), false);
  assert.equal(couldBeChannel('two words'), false);
  assert.equal(couldBeChannel('rvb-dev'), false);
  assert.equal(couldBeChannel('???'), false);
  assert.equal(couldBeChannel('https://www.twitch.tv/rvbdev'), true);
  assert.equal(couldBeChannel('twitch.tv/ab'), true);
  assert.equal(couldBeChannel('https://www.twitch.tv/videos/123'), false);
});

test('manyNames tells a pasted list from one name', () => {
  assert.equal(manyNames('rvbdev'), false);
  assert.equal(manyNames('  rvbdev  '), false);
  assert.equal(manyNames(''), false);
  assert.equal(manyNames('rvbdev xqc'), true);
  assert.equal(manyNames('rvbdev,xqc'), true);
  assert.equal(manyNames('rvbdev, xqc, shroud'), true);
  assert.equal(manyNames('rvbdev,'), false);
});
