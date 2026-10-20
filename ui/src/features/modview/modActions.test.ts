import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ModActionFilters } from '../../ipc/bindings/ModActionFilters';
import type { ModLine } from '../../ipc/bindings/ModLine';
import { ACTION_KINDS, moderatorsIn, visibleActions } from './modActions.ts';

const all: ModActionFilters = {
  bans: true,
  timeouts: true,
  warnings: true,
  deletions: true,
  modes: true,
  roles: true,
  automod: true,
  other: true,
  date: true,
  time: true,
};

function line(id: string, group: ModLine['group'], mod: string, target: string | null): ModLine {
  return {
    id,
    ts: 1,
    group,
    action: 'did',
    moderator: { login: mod, name: mod.toUpperCase() },
    target: target ? { login: target, name: target === 'zed' ? 'Zed_42' : target } : null,
    secs: null,
    reason: '',
    about: '',
    from: null,
    past: false,
  };
}

const lines = [line('1', 'ban', 'kuni', 'zed'), line('2', 'timeout', 'amy', 'bob'), line('3', 'mode', 'kuni', null), line('4', 'automod', 'amy', 'zed')];
const ids = (l: ModLine[]) => l.map((x) => x.id).join();

test('kinds turned off drop out', () => {
  assert.equal(ids(visibleActions(lines, { filters: all, moderator: '', user: '' })), '1,2,3,4');
  assert.equal(ids(visibleActions(lines, { filters: { ...all, bans: false, modes: false }, moderator: '', user: '' })), '2,4');
  // Every group has a chip.
  assert.equal(new Set(ACTION_KINDS.map((k) => k.group)).size, 8);
});

test('by moderator and by user, by login or shown name', () => {
  assert.equal(ids(visibleActions(lines, { filters: all, moderator: 'kuni', user: '' })), '1,3');
  assert.equal(ids(visibleActions(lines, { filters: all, moderator: '', user: '@ZED' })), '1,4');
  assert.equal(ids(visibleActions(lines, { filters: all, moderator: '', user: '_42' })), '1,4');
  // A line about nobody never matches a user.
  assert.equal(ids(visibleActions(lines, { filters: all, moderator: 'kuni', user: 'z' })), '1');
});

test('moderators listed once, A-Z by name', () => {
  assert.deepEqual(moderatorsIn(lines), [
    { login: 'amy', name: 'AMY' },
    { login: 'kuni', name: 'KUNI' },
  ]);
});
