import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Moment } from '../../ipc/bindings/Moment.ts';
import { canGoThere, clipOffset, hereClip, KEEP, LIVE_CLIP_REACH, momentClip, marks, mergeMoments, momentText, newest, playingOffset, secsOf } from './moments.ts';

const m = (id: number, over: Partial<Moment> = {}): Moment => ({
  id,
  atMs: 1_000_000 + id * 1000,
  streamSecs: id * 60,
  kind: 'spike',
  label: '',
  score: 0,
  ...over,
});

test('the text names the kind and the label, cut when long', () => {
  assert.equal(momentText({ kind: 'spike', label: 'KEKW' }), 'Chat spike · KEKW');
  assert.equal(momentText({ kind: 'raid', label: '' }), 'Raid');
  assert.equal(momentText({ kind: 'gift', label: '  a   b ' }), 'Gift bomb · a b');
  const long = momentText({ kind: 'announcement', label: 'x'.repeat(100) });
  assert.ok(long.endsWith('…') && long.length < 60);
  assert.equal(momentText({ kind: 'nope' as never, label: '' }), 'Moment');
  // Emoji are not split in half.
  assert.ok(!momentText({ kind: 'spike', label: '😀'.repeat(60) }).includes('�'));
});

test('seconds come from the chat, else from the stream start', () => {
  assert.equal(secsOf({ atMs: 5000, streamSecs: 42 }, null), 42);
  assert.equal(secsOf({ atMs: 70_000, streamSecs: null }, 10_000), 60);
  assert.equal(secsOf({ atMs: 70_000, streamSecs: null }, null), null);
  assert.equal(secsOf({ atMs: 1000, streamSecs: null }, 10_000), 0);
});

test('go there needs a source that covers the second', () => {
  const src = { on: true, min: 600, uptime: 3600 };
  assert.equal(canGoThere(900, src), true);
  assert.equal(canGoThere(599, src), false);
  assert.equal(canGoThere(3601, src), false);
  assert.equal(canGoThere(null, src), false);
  assert.equal(canGoThere(900, { ...src, on: false }), false);
  assert.equal(canGoThere(0, { on: true, min: 0, uptime: 100 }), true);
});

test('a clip ends the lead after the moment, never past now', () => {
  assert.equal(clipOffset(100, 500), 110);
  assert.equal(clipOffset(495, 500), 500);
  assert.equal(clipOffset(-20, 500), 0);
});

test('a recent moment clips live; an old one needs the archive video, else no clip', () => {
  assert.deepEqual(momentClip(450, 500, null), { kind: 'live', offset: 460 });
  assert.deepEqual(momentClip(100, 5000, 'v1'), { kind: 'vod', video: 'v1', offset: 110 });
  assert.equal(momentClip(100, 5000, null), null);
  assert.equal(momentClip(5000 - LIVE_CLIP_REACH, 5000, null)?.kind, 'live');
});

test('V108-N25 a rewound clip ends where the player is', () => {
  assert.deepEqual(hereClip(4950.6, 5000, null), { kind: 'live', offset: 4950 });
  assert.deepEqual(hereClip(1000, 5000, 'v1'), { kind: 'vod', video: 'v1', offset: 1000 });
  assert.equal(hereClip(1000, 5000, null), null);
});

test('merging keeps ids once, in order, newest 200', () => {
  const a = [m(1), m(2)];
  assert.equal(mergeMoments(a, [m(2)]), a);
  assert.deepEqual(mergeMoments(a, [m(4), m(3)]).map((x) => x.id), [1, 2, 3, 4]);
  const many = Array.from({ length: KEEP + 5 }, (_, i) => m(i + 1));
  const kept = mergeMoments([], many);
  assert.equal(kept.length, KEEP);
  assert.equal(kept[0]!.id, 6);
});

test('marks sit from the bar start and skip what cannot be reached', () => {
  const list = [m(1), m(5), m(30), m(70, { streamSecs: null, atMs: 4_700_000 })];
  const src = { on: true, min: 120, uptime: 3600 };
  // Bar starts at 120 s: the moment at 60 s is before it.
  const got = marks(list, 500_000, src, 120);
  // 70 is past now (4.200 s of a 3.600 s stream); 1 is before the bar.
  assert.deepEqual(got.map((x) => [x.id, x.t]), [[5, 180], [30, 1680]]);
  assert.equal(marks(list, null, { ...src, on: false }, 0).length, 0);
  assert.match(got[0]!.text, /^Chat spike · 5:00$/);
});

test('the menu lists the newest first', () => {
  const list = Array.from({ length: 40 }, (_, i) => m(i + 1));
  const got = newest(list);
  assert.equal(got.length, 30);
  assert.equal(got[0]!.id, 40);
  assert.equal(got.at(-1)!.id, 11);
});

test('V109-F73 the live clip ends on the frame on screen, by Twitch’s clock', () => {
  const origin = Date.parse('2026-11-07T10:00:00Z');
  assert.equal(playingOffset(origin + 4_000_500, origin), 4000.5);
  assert.equal(playingOffset(null, origin), null, 'no date: the app’s now');
  assert.equal(playingOffset(origin + 1000, 0), null, 'no start');
  assert.equal(playingOffset(origin - 1000, origin), null);
});
