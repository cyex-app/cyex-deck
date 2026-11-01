import assert from 'node:assert/strict';
import { test } from 'node:test';

import { shown, streakShort, streakText } from './streakShare.ts';

test('V108-N23 row words', () => {
  assert.equal(streakText(5), '5-stream watch streak');
  assert.equal(streakText(1234, () => '1.234'), '1.234-stream watch streak');
  assert.equal(streakText(0), 'Watch streak');
  assert.equal(streakShort(5), '5-stream streak');
  assert.equal(streakShort(1234, () => '1.234'), '1.234-stream streak');
  assert.equal(streakShort(0), 'Streak');
});

test('V108-N23 only a shareable streak for this chat shows', () => {
  const m = (value: number, shareStatus = 'CAN_SHARE', id = `s${value}`) => ({ id, category: 'WATCH_STREAK', value, state: 'ACTIVE', shareStatus });
  const chat = { account: '1', login: 'Chan', channelId: '9' };
  const none = () => false;
  const ans = (milestone: ReturnType<typeof m> | null, channelId: string | null = '9') => ({ milestone, channelId, live: true });
  assert.equal(shown(ans(m(3)), chat, chat, none)?.value, 3);
  assert.equal(shown(ans(m(5, 'CAN_NOT_SHARE')), chat, chat, none), null);
  assert.equal(shown(ans(m(0)), chat, chat, none), null);
  assert.equal(shown(ans({ ...m(3), category: 'SUBSCRIBER_ANNIVERSARY' }), chat, chat, none), null);
  // Another channel's answer, a missing channel, or an answer for the account or chat before.
  assert.equal(shown(ans(m(3), '8'), chat, chat, none), null);
  assert.equal(shown(ans(m(3), null), chat, chat, none), null);
  assert.equal(shown(ans(m(3)), { ...chat, account: '2' }, chat, none), null);
  assert.equal(shown(ans(m(3)), { ...chat, login: 'other' }, chat, none), null);
  // Shared or waved off.
  assert.equal(shown(ans(m(3)), chat, chat, (id) => id === 's3'), null);
  // Share 3, then an event says 5: the row waits for the read; a read without a shareable 5 shows nothing.
  const waved = new Set(['s3']);
  assert.equal(shown(ans(m(3, 'SHARED')), chat, chat, (id) => waved.has(id)), null);
  assert.equal(shown(ans(null), chat, chat, (id) => waved.has(id)), null);
  assert.equal(shown(ans(m(5)), chat, chat, (id) => waved.has(id))?.value, 5);
});
