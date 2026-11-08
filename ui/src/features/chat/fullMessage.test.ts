import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { fullMessage } from './fullMessage.ts';

const ts = new Date(2026, 10, 8, 14, 3, 9).getTime();
const base = { ts, flags: [], body: [{ t: 'text', text: 'hi ' }, { t: 'emote', name: 'Kappa' }], sender: { login: 'sai', display: 'サイ' } };
const msg = (o: object) => ({ ...base, ...o }) as unknown as ChatMessage;
const h24 = { seconds: false, clock: 'h24' as const };

test('time, name, colon, text with emotes as names', () => {
  assert.equal(fullMessage(msg({}), 'display', h24), '[14:03] サイ: hi Kappa');
});

test('the name follows the Names setting and a nickname wins', () => {
  assert.equal(fullMessage(msg({}), 'username', h24), '[14:03] sai: hi Kappa');
  assert.equal(fullMessage(msg({}), 'display', h24, { nickname: 'Sai', note: '' }), '[14:03] Sai: hi Kappa');
});

test('seconds and 12-hour follow the timestamp settings', () => {
  assert.equal(fullMessage(msg({}), 'username', { seconds: true, clock: 'h24' }), '[14:03:09] sai: hi Kappa');
  assert.equal(fullMessage(msg({}), 'username', { seconds: false, clock: 'h12' }), '[2:03 pm] sai: hi Kappa');
});

test('/me has no colon', () => {
  assert.equal(fullMessage(msg({ flags: ['action'] }), 'username', h24), '[14:03] sai hi Kappa');
});

test('a line with no sender is time and text', () => {
  const n = msg({ sender: undefined, body: [], notice: { kind: 'info', text: 'Chat cleared' } });
  assert.equal(fullMessage(n, 'username', h24), '[14:03] Chat cleared');
});
