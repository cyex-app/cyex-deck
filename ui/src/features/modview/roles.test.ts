import assert from 'node:assert/strict';
import { test } from 'node:test';

import { roleOf } from './roles.ts';

test('badges give a chatter their role', () => {
  assert.equal(roleOf(undefined), null);
  assert.equal(roleOf([{ set: 'vip' }, { set: 'subscriber' }]), null);
  assert.equal(roleOf([{ set: 'moderator' }]), 'moderator');
  assert.equal(roleOf([{ set: 'moderator' }, { set: 'lead_moderator' }]), 'leadModerator');
  assert.equal(roleOf([{ set: 'broadcaster' }]), 'broadcaster');
});
