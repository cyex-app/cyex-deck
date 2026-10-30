import assert from 'node:assert/strict';
import { test } from 'node:test';

import { logsMessageUrl, logsUserUrl } from './logsLink.ts';

const id = '7a1b0c3e-1111-2222-3333-444455556666';

test('channel, UTC day and the message anchor', () => {
  // 2026/10/29 23:30 UTC is already the 30th in Europe: the day stays UTC.
  const ts = Date.UTC(2026, 9, 29, 23, 30);
  assert.equal(logsMessageUrl('xqc', { id, ts }), `https://logs.cyex.app/?c=xqc&d=2026-10-29#msg-${id}`);
});

test('shared chat points at the channel it was sent in', () => {
  const ts = Date.UTC(2026, 0, 2);
  assert.equal(
    logsMessageUrl('xqc', { id, ts, source: { roomId: '123', badges: [] } }),
    `https://logs.cyex.app/?c=id%3A123&d=2026-01-02#msg-${id}`,
  );
});

test('no link for local lines, missing times or channels', () => {
  assert.equal(logsMessageUrl('xqc', { id: 'local-5', ts: 1 }), null);
  assert.equal(logsMessageUrl('xqc', { id, ts: 0 }), null);
  assert.equal(logsMessageUrl('', { id, ts: 1 }), null);
});

test('a user page, by login or by room id (V108-N17)', () => {
  assert.equal(logsUserUrl('xqc', 'viewer1'), 'https://logs.cyex.app/?c=xqc&u=viewer1');
  assert.equal(logsUserUrl('id:123', 'viewer1'), 'https://logs.cyex.app/?c=id%3A123&u=viewer1');
  assert.equal(logsUserUrl('', 'viewer1'), null);
});
