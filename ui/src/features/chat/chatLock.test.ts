import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChatStanding } from '../../ipc/bindings/ChatStanding';
import { headline, left, lockedWhy, refocusBox, ticking, title, lockOf, nextRead, unbanLine, unbanOf, type Lock, type LockInput } from './chatLock.ts';

const NOW = Date.parse('2026-11-04T12:00:00Z');
const iso = (ms: number) => new Date(ms).toISOString();
const banned = (sinceAgoMin: number, extra: Partial<ChatStanding> = {}): ChatStanding => ({
  ban: { until: null, by: 'Kuni', reason: 'spam', since: iso(NOW - sinceAgoMin * 60_000) },
  unban: { enabled: true, cooldownMinutes: 180 },
  canRequest: null,
  request: null,
  ...extra,
});
const input = (p: Partial<LockInput>): LockInput => ({ standing: null, readAt: 0, said: null, warning: null, now: NOW, ...p });

test('free: no lock', () => {
  assert.equal(lockOf(input({})), null);
  assert.equal(lockOf(input({ standing: { ban: null, unban: null, canRequest: null, request: null } })), null);
});

test('a timeout counts down and unlocks by itself', () => {
  const st: ChatStanding = { ban: { until: iso(NOW + 299_500), by: 'Kuni', reason: 'calm down' }, unban: null, canRequest: null, request: null };
  const lock = lockOf(input({ standing: st, readAt: NOW - 1000 }))!;
  assert.equal(lock.kind, 'timeout');
  assert.equal(headline(lock, NOW), 'Timed out · 5:00');
  assert.equal(headline(lock, NOW + 1_000), 'Timed out · 4:59');
  assert.equal(lockOf(input({ standing: st, now: NOW + 300_000 })), null);
});

test('timer text like the pin timer, a date from a day on', () => {
  assert.equal(left(NOW + 3_723_000, NOW), '1:02:03');
  assert.equal(left(NOW + 59_000, NOW), '0:59');
  assert.match(left(NOW + 2 * 86_400_000, NOW), /^\d{4}\/\d{2}\/\d{2} \d{2}:\d{2}$/);
});

test('IRC locks at once, until a later read says otherwise', () => {
  const said = { how: { kind: 'timedOut' as const, secs: 60 }, at: NOW };
  const lock = lockOf(input({ said }))!;
  assert.equal(lock.kind === 'timeout' && lock.until, NOW + 60_000);
  // A read that started right after may not know yet: IRC still counts.
  assert.equal(lockOf(input({ said, readAt: NOW + 500 }))?.kind, 'timeout');
  assert.equal(lockOf(input({ said, readAt: NOW + 4_000 })), null);
  const ban = lockOf(input({ said: { how: { kind: 'banned' }, at: NOW } }))!;
  assert.equal(ban.kind, 'ban');
  assert.equal(ban.kind === 'ban' && ban.reason, '');
});

test('the unban request opens after the cooldown', () => {
  assert.deepEqual(unbanOf(banned(60), NOW - 3_600_000, NOW), { state: 'wait', at: NOW + 2 * 3_600_000 });
  assert.equal(unbanLine({ state: 'wait', at: NOW + 8_100_000 }, NOW), 'Available in 2:15:00');
  assert.equal(unbanOf(banned(200), NOW - 200 * 60_000, NOW).state, 'ready');
  assert.equal(unbanOf(banned(10, { canRequest: true }), NOW - 600_000, NOW).state, 'ready', 'Twitch says yes');
  assert.equal(unbanOf(banned(200, { canRequest: false }), NOW - 200 * 60_000, NOW).state, 'closed');
  // V108-C40f A "no" read during the wait doesn't close it once the wait is over.
  assert.equal(unbanOf(banned(200, { canRequest: false }), NOW - 200 * 60_000, NOW, NOW - 30 * 60_000).state, 'ready');
  assert.equal(unbanOf(banned(200, { canRequest: false }), NOW - 200 * 60_000, NOW, NOW - 10 * 60_000).state, 'closed');
  assert.equal(unbanOf(banned(200, { unban: { enabled: false, cooldownMinutes: 0 } }), null, NOW).state, 'off');
  assert.equal(unbanOf(null, null, NOW).state, 'off');
});

test('a pending request stops a second one; a denied one shows its note', () => {
  const req = { id: 'r', createdAt: iso(NOW), text: 'sorry', status: 'PENDING', resolvedAt: null, resolvedBy: null, note: '' };
  const pending = unbanOf(banned(300, { request: req, canRequest: true }), NOW - 300 * 60_000, NOW);
  assert.equal(pending.state, 'pending');
  assert.match(unbanLine(pending, NOW), /^Unban request sent 2026\/11\/0\d\. Waiting for a mod\.$/);
  const denied = unbanOf(banned(300, { request: { ...req, status: 'DENIED', note: 'Not now' }, canRequest: false }), NOW - 300 * 60_000, NOW);
  assert.equal(unbanLine(denied, NOW), 'Unban request denied: Not now');
});

test('a ban outranks a timeout, a timeout a warning', () => {
  const warning = { reason: 'be nice', rules: ['No spam'] };
  assert.equal(lockOf(input({ standing: banned(5), warning }))?.kind, 'ban');
  const said = { how: { kind: 'timedOut' as const, secs: 30 }, at: NOW };
  assert.equal(lockOf(input({ said, warning }))?.kind, 'timeout');
  const w = lockOf(input({ warning }))!;
  assert.equal(w.kind === 'warn' && w.warning.reason, 'be nice');
  assert.equal(lockOf(input({ said: { how: { kind: 'warned' }, at: NOW } }))?.kind, 'warn');
});

test('reads again when it matters', () => {
  const ban = lockOf(input({ standing: banned(300, { canRequest: true }) }));
  assert.equal(nextRead(ban, input({})), 180_000);
  const wait = lockOf(input({ standing: banned(178) }));
  assert.equal(nextRead(wait, input({})), 121_000, 'right when it opens');
  assert.equal(nextRead(null, input({})), null);
  const said = { how: { kind: 'banned' as const }, at: NOW };
  assert.equal(nextRead(null, input({ said, readAt: NOW + 100 })), 5_000, 'the read came too soon');
});

test('the status line is static, the time ticks apart from it', () => {
  const st: ChatStanding = { ban: { until: iso(NOW + 299_500), by: 'Kuni', reason: '' }, unban: null, canRequest: null, request: null };
  const lock = lockOf(input({ standing: st, readAt: NOW - 1000 }))!;
  assert.equal(title(lock), 'Timed out');
  assert.equal(title(lock), title(lock));
  assert.equal(ticking(lock, NOW), '5:00');
  assert.equal(ticking(lock, NOW + 1_000), '4:59');
  const ban = lockOf(input({ standing: banned(1) }))!;
  assert.equal(title(ban), 'You’re banned here.');
  assert.equal(ticking(ban, NOW), '');
});

test('the box takes focus back only when it had it or nothing has', () => {
  assert.equal(refocusBox(true, false), true);
  assert.equal(refocusBox(false, true), true);
  assert.equal(refocusBox(false, false), false);
});

test('lockedWhy names the lock, and is empty without one', () => {
  const warn: Lock = { kind: 'warn', warning: { reason: '', rules: [] } };
  const timeout: Lock = { kind: 'timeout', until: NOW + 1000, reason: '' };
  const ban: Lock = { kind: 'ban', reason: '', since: null, unban: { state: 'off' } };
  assert.equal(lockedWhy(null), '');
  assert.equal(lockedWhy(timeout), 'Not while you’re timed out.');
  assert.equal(lockedWhy(ban), 'Not while you’re banned.');
  assert.equal(lockedWhy(warn), 'Not until you acknowledge the warning.');
});
