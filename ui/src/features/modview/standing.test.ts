import assert from 'node:assert/strict';
import { test } from 'node:test';

import { roleItems, standingOf } from './standing.ts';

const labels = (is?: { mod: boolean; vip: boolean }) =>
  roleItems('bob', () => {}, is).map((e) => (typeof e === 'string' ? e : e.label));

test('badges tell mods and VIPs apart (M12)', () => {
  assert.equal(standingOf(undefined), undefined);
  assert.deepEqual(standingOf([{ set: 'subscriber' }]), { mod: false, vip: false });
  assert.deepEqual(standingOf([{ set: 'lead_moderator' }]), { mod: true, vip: false });
  assert.deepEqual(standingOf([{ set: 'vip' }, { set: 'subscriber' }]), { mod: false, vip: true });
});

test('name menus offer what can still change (M12)', () => {
  assert.deepEqual(labels(), ['Mod', 'VIP']);
  assert.deepEqual(labels({ mod: false, vip: false }), ['Mod', 'VIP']);
  assert.deepEqual(labels({ mod: true, vip: false }), ['Unmod']);
  assert.deepEqual(labels({ mod: false, vip: true }), ['Mod', 'Remove VIP']);
  const ran: string[] = [];
  for (const e of roleItems('bob', (l) => ran.push(l), { mod: false, vip: true })) if (typeof e !== 'string') e.run?.();
  assert.deepEqual(ran, ['/mod bob', '/unvip bob']);
});
