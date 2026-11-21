import assert from 'node:assert/strict';
import { test } from 'node:test';

import { isModSet, roleOf } from './roles.ts';

test('badges give a chatter their role', () => {
  assert.equal(roleOf(undefined), null);
  assert.equal(roleOf([{ set: 'vip' }, { set: 'subscriber' }]), null);
  assert.equal(roleOf([{ set: 'moderator' }]), 'moderator');
  assert.equal(roleOf([{ set: 'moderator' }, { set: 'lead_moderator' }]), 'leadModerator');
  assert.equal(roleOf([{ set: 'broadcaster' }]), 'broadcaster');
});

test('V112-21 lead mods are mods, alone or with the mod badge', () => {
  assert.equal(roleOf([{ set: 'lead_moderator' }]), 'leadModerator');
  assert.equal(roleOf([{ set: 'subscriber' }, { set: 'lead_moderator' }]), 'leadModerator');
  assert.ok(isModSet('lead_moderator') && isModSet('moderator'));
  assert.ok(!isModSet('vip') && !isModSet('broadcaster'));
});
