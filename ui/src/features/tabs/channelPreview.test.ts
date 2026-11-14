import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
import { previewFacts, previewUrl, previewWidth } from './channelPreview.ts';

const base: ChannelInfo = { id: '1', login: 'xqc', name: 'xQc', avatar: null, offlineImage: null, stream: null };

test('the width covers the box at the screen density', () => {
  assert.equal(previewWidth(180, 1), 192);
  assert.equal(previewWidth(180, 1.5), 320);
  assert.equal(previewWidth(180, 2), 440);
  assert.equal(previewWidth(400, 2), 640);
  assert.equal(previewWidth(180, 0), 192);
});

test('the url is 16:9 and changes every minute', () => {
  const a = previewUrl('XQC', 180, 1, 120_000);
  assert.equal(a, 'https://static-cdn.jtvnw.net/previews-ttv/live_user_xqc-192x108.jpg?t=2');
  assert.equal(previewUrl('xqc', 180, 1, 179_999), a);
  assert.notEqual(previewUrl('xqc', 180, 1, 180_000), a);
});

test('a live channel shows thumbnail, viewers and uptime', () => {
  const now = Date.parse('2026-11-15T13:30:00Z');
  const f = previewFacts(
    { ...base, stream: { rerun: false, title: ' Hello ', game: 'Chess', viewers: 12345, startedAt: '2026-11-15T10:00:00Z' } },
    180,
    1,
    now,
  );
  assert.equal(f.live, true);
  assert.equal(f.title, 'Hello');
  assert.equal(f.game, 'Chess');
  assert.equal(f.viewers, '12.345 watching');
  assert.equal(f.uptime, '3h 30m');
  assert.ok(f.image?.includes('live_user_xqc-192x108'));
});

test('an offline channel keeps its banner or none', () => {
  assert.equal(previewFacts(base, 180, 1).image, null);
  const f = previewFacts({ ...base, offlineImage: 'https://x/o.png' }, 180, 1);
  assert.equal(f.live, false);
  assert.equal(f.image, 'https://x/o.png');
});
