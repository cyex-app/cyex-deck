import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Clip } from '../../ipc/bindings/Clip';
import type { ManagedVideo } from '../../ipc/bindings/ManagedVideo';
import { bulkLines, clipLine, clipLink, count, kindOf, length, videoLine, videoLink, videoTag } from './content.ts';

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
  assert.match(clipLine(clip, false), /^by Someone · 2026\/10\/\d\d · 1\.200\u00a0views$/);
  assert.match(clipLine(clip, true), /^in cyexbot · /);
  assert.equal(clipLine({ ...clip, clippedBy: null, createdAt: 'x', views: 1 }, false), '1\u00a0view');
  assert.equal(clipLink(clip), 'https://www.twitch.tv/cyexbot/clip/Fast-Clip');
  assert.equal(clipLink({ ...clip, channelLogin: '' }), 'https://clips.twitch.tv/Fast-Clip');
});

test('video lines and tags', () => {
  assert.match(videoLine(video), /^Broadcast · 2026\/10\/\d\d · 1\u00a0view$/);
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

test('counts', () => {
  assert.equal(count(1, 'clip', 'clips'), '1 clip');
  assert.equal(count(1000, 'clip', 'clips'), '1.000 clips');
});

test('V107-R41 bulk results: what went, then stopped or why not', () => {
  const did = (n: number) => `${n} gone.`;
  assert.deepEqual(bulkLines({ done: ['a', 'b'], error: null, stopped: false }, 2, did), { ok: '2 gone.', info: null, bad: null });
  assert.deepEqual(bulkLines({ done: ['a'], error: 'Too fast.', stopped: false }, 3, did), { ok: '1 gone.', info: null, bad: 'Too fast.' });
  assert.equal(bulkLines({ done: [], error: null, stopped: false }, 2, did).bad, 'Twitch kept them. Try again.');
  assert.equal(bulkLines({ done: [], error: null, stopped: false }, 1, did).bad, 'Twitch kept it. Try again.');
  assert.equal(bulkLines({ done: ['a'], error: null, stopped: false }, 2, did).bad, 'Twitch kept some of them. Try again.');
  const stopped = bulkLines({ done: ['a'], error: null, stopped: true }, 3, did);
  assert.equal(stopped.bad, null);
  assert.match(stopped.info ?? '', /Stopped/);
});
