import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Clip } from '../../ipc/bindings/Clip';
import type { ManagedVideo } from '../../ipc/bindings/ManagedVideo';
import { appended, clipLine, clipLink, count, kept, kindOf, length, toggled, videoLine, videoLink, videoTag } from './content.ts';

const clip: Clip = {
  slug: 'Fast-Clip',
  title: 'gg',
  views: 1200,
  createdAt: '2026-10-20T10:00:00Z',
  duration: 30,
  thumbnail: null,
  channelLogin: 'cyexbot',
  channelName: 'cyexbot',
  clippedBy: 'Someone',
  game: null,
};

const video: ManagedVideo = {
  id: '7',
  title: 'Stream',
  kind: 'archive',
  length: 3723,
  date: '2026-10-20T10:00:00Z',
  views: 1,
  public: true,
  state: 'ready',
  thumbnail: null,
  game: null,
};

test('clip lines name who clipped it, or the channel for your own clips', () => {
  assert.match(clipLine(clip, false), /^by Someone · 2026\/10\/\d\d · 1\.200 views$/);
  assert.match(clipLine(clip, true), /^in cyexbot · /);
  assert.equal(clipLine({ ...clip, clippedBy: null, createdAt: 'x', views: 1 }, false), '1 view');
  assert.equal(clipLink(clip), 'https://www.twitch.tv/cyexbot/clip/Fast-Clip');
  assert.equal(clipLink({ ...clip, channelLogin: '' }), 'https://clips.twitch.tv/Fast-Clip');
});

test('video lines and tags', () => {
  assert.match(videoLine(video), /^Broadcast · 2026\/10\/\d\d · 1 view$/);
  assert.equal(videoTag(video), null);
  assert.equal(videoTag({ ...video, public: false }), 'Unpublished');
  assert.equal(videoTag({ ...video, state: 'recording' }), 'Live');
  assert.equal(videoTag({ ...video, state: 'processing', public: false }), 'Processing');
  assert.equal(videoLink('7'), 'https://www.twitch.tv/videos/7');
  assert.equal(length(3723), '1:02:03');
  assert.equal(length(0), '');
  assert.equal(kindOf('all'), null);
  assert.equal(kindOf('highlight'), 'highlight');
});

test('selection flips, and drops what left the list', () => {
  const a = toggled(new Set(), 'x');
  assert.deepEqual([...a], ['x']);
  assert.deepEqual([...toggled(a, 'x')], []);
  assert.deepEqual([...kept(new Set(['x', 'y']), ['y', 'z'])], ['y']);
});

test('more rows append without repeats', () => {
  const k = (n: { id: string }) => n.id;
  assert.deepEqual(appended([{ id: 'a' }], [{ id: 'a' }, { id: 'b' }], k), [{ id: 'a' }, { id: 'b' }]);
  assert.equal(count(1, 'clip', 'clips'), '1 clip');
  assert.equal(count(1000, 'clip', 'clips'), '1.000 clips');
});
