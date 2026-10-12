import assert from 'node:assert/strict';
import { test } from 'node:test';

import { allows, roleOf } from './roles.ts';

test('each role reaches what it should (V102-21)', () => {
  const table: [Parameters<typeof allows>[0], boolean, boolean, boolean][] = [
    [null, false, false, false],
    ['moderator', true, false, false],
    ['leadModerator', true, true, false],
    ['broadcaster', true, true, true],
  ];
  for (const [role, mod, lead, owner] of table) {
    assert.equal(allows(role, 'mod'), mod, `${role} mod`);
    assert.equal(allows(role, 'lead'), lead, `${role} lead`);
    assert.equal(allows(role, 'broadcaster'), owner, `${role} broadcaster`);
  }
});

test('badges give a chatter their role', () => {
  assert.equal(roleOf(undefined), null);
  assert.equal(roleOf([{ set: 'vip' }, { set: 'subscriber' }]), null);
  assert.equal(roleOf([{ set: 'moderator' }]), 'moderator');
  assert.equal(roleOf([{ set: 'moderator' }, { set: 'lead_moderator' }]), 'leadModerator');
  assert.equal(roleOf([{ set: 'broadcaster' }]), 'broadcaster');
});
