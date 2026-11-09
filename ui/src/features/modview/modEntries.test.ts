import assert from 'node:assert/strict';
import { test } from 'node:test';

import { banEntry, modRoleEntry, type ModUser, pinItems, timeoutEntry, timeoutItems, vipRoleEntry } from './modEntries.ts';

const user = (patch: Partial<ModUser> = {}, ran: string[] = []): ModUser => ({
  login: 'bob',
  roles: true,
  onMod: { offered: true },
  presets: [{ label: '10s', secs: 10 }],
  run: (l) => void ran.push(l),
  warn: () => {},
  ...patch,
});

test('time out and ban follow the mod-target rule', () => {
  const off = user({ onMod: { offered: false } });
  assert.equal(timeoutEntry(off), null);
  assert.equal(banEntry(off), null);
  const ran: string[] = [];
  const ban = banEntry(user({}, ran));
  assert.ok(ban && ban.danger && ban.confirm);
  ban.run?.();
  assert.deepEqual(ran, ['/ban bob']);
});

test('mod and vip rows need the roles cap and follow the standing', () => {
  assert.equal(modRoleEntry(user({ roles: false })), null);
  assert.equal(vipRoleEntry(user({ roles: false })), null);
  const m = modRoleEntry(user({ standing: { mod: true, vip: false } }));
  assert.ok(m && m !== 'separator');
  assert.equal(m.label, 'Unmod');
});

test('timeout lengths: presets, then a typed one', () => {
  const ran: string[] = [];
  const items = timeoutItems('bob', (l) => void ran.push(l), [{ label: '10s', secs: 10 }]);
  const last = items.at(-1);
  assert.ok(last && last !== 'separator' && last.field);
  assert.equal(last.field.submit('7m'), null);
  assert.ok(last.field.submit('nope'));
  assert.deepEqual(ran, ['/timeout bob 420']);
});

test('pin lengths end with a typed one', () => {
  const ran: string[] = [];
  const items = pinItems('m1', (l) => void ran.push(l));
  const first = items[0];
  assert.ok(first !== 'separator');
  first.run?.();
  assert.deepEqual(ran, ['/pin m1']);
});
