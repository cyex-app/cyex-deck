import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { UserRule } from '../../ipc/bindings/UserRule';
import { toggleWatch, watched } from './watch.ts';

const rule = (login: string, extra: Partial<UserRule> = {}): UserRule => ({
  enabled: true,
  login,
  regex: false,
  color: '',
  sound: false,
  soundFile: '',
  flash: false,
  show: false,
  route: '',
  ...extra,
});

test('the bell adds a plain highlight and takes it back', () => {
  const on = toggleWatch([], 'Forsen');
  assert.deepEqual(on, [rule('forsen')]);
  assert.ok(watched(on, 'forsen'));
  const off = toggleWatch(on, 'forsen');
  assert.equal(off.length, 1);
  assert.ok(!watched(off, 'forsen'));
});

test('the bell keeps an existing rule and ignores regex rules', () => {
  const users = [rule('^for.*', { regex: true }), rule('forsen', { enabled: false, sound: true })];
  assert.ok(!watched(users, 'forsen'));
  const on = toggleWatch(users, 'forsen');
  assert.deepEqual(on[0], users[0]);
  assert.deepEqual(on[1], { ...users[1], enabled: true });
});
