import assert from 'node:assert/strict';
import { test } from 'node:test';

import { applyChange, merged, roleItems, standingOf } from './standing.ts';

const labels = (is?: { mod: boolean; vip: boolean }) =>
  roleItems('bob', () => {}, is).map((e) => (typeof e === 'string' ? e : e.label));

test('badges tell mods and VIPs apart (M12)', () => {
  assert.equal(standingOf(undefined), undefined);
  assert.deepEqual(standingOf([{ set: 'subscriber' }]), { mod: false, vip: false });
  assert.deepEqual(standingOf([{ set: 'lead_moderator' }]), { mod: true, vip: false });
  assert.deepEqual(standingOf([{ set: 'vip' }, { set: 'subscriber' }]), { mod: false, vip: true });
});

test('menus offer Mod or Unmod and VIP or Unvip (V107-M8)', () => {
  assert.deepEqual(labels(), ['Mod', 'VIP']);
  assert.deepEqual(labels({ mod: false, vip: false }), ['Mod', 'VIP']);
  assert.deepEqual(labels({ mod: true, vip: false }), ['Unmod', 'VIP']);
  assert.deepEqual(labels({ mod: false, vip: true }), ['Mod', 'Unvip']);
  const ran: string[] = [];
  for (const e of roleItems('bob', (l) => ran.push(l), { mod: false, vip: true })) if (typeof e !== 'string') e.run?.();
  assert.deepEqual(ran, ['/mod bob', '/unvip bob']);
});

test('a change wins over badges, granting one clears the other (V107-M8)', () => {
  const vip = { mod: false, vip: true };
  assert.equal(merged(undefined, undefined), undefined);
  assert.deepEqual(merged(vip, undefined), vip);
  const modded = applyChange(undefined, { moderator: true, on: true });
  assert.deepEqual(merged(vip, modded), { mod: true, vip: false });
  const vipped = applyChange(modded, { moderator: false, on: true });
  assert.deepEqual(merged(vip, vipped), { mod: false, vip: true });
  const unvipped = applyChange(vipped, { moderator: false, on: false });
  assert.deepEqual(merged(vip, unvipped), { mod: false, vip: false });
  // Taking one away says nothing about the other.
  assert.deepEqual(merged(vip, applyChange(undefined, { moderator: true, on: false })), vip);
  assert.deepEqual(merged(undefined, applyChange(undefined, { moderator: true, on: false })), { mod: false, vip: false });
});
