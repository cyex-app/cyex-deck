import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ModsetVerification } from '../../ipc/bindings/ModsetVerification';
import { levelsFor, minuteChoices, ruleProblem, withMode, withoutRule, withPartial, withRule } from './modset.ts';

const off = { mode: 'NONE' as const, partial: { firstTime: false, accountYoungerThan: null, followingLessThan: null } };
const V: ModsetVerification = { email: off, phone: off, exemptSubs: false, exemptVips: false, exemptMods: false };

test('smart detection takes off, 2 and 4 only', () => {
  assert.deepEqual(levelsFor(9).map((l) => l.value), ['0', '2', '4']);
  assert.equal(levelsFor(0).length, 5);
});

test('a length set on Twitch shows as itself', () => {
  assert.deepEqual(minuteChoices([10, 60], 30).map((c) => c.value), ['10', '30', '60']);
  assert.deepEqual(minuteChoices([10, 60], null).map((c) => c.value), ['10', '60']);
  assert.equal(minuteChoices([0, 60], 0)[0].label, 'None');
  assert.equal(minuteChoices([1440], null)[0].label, '1d');
});

test('verification edits touch one side', () => {
  const v = withPartial(withMode(V, 'phone', 'PARTIAL'), 'phone', { followingLessThan: 10 });
  assert.equal(v.phone.mode, 'PARTIAL');
  assert.equal(v.phone.partial.followingLessThan, 10);
  assert.equal(v.email, off);
  assert.equal(V.phone.mode, 'NONE');
});

test('rules', () => {
  assert.equal(ruleProblem('  ', []), 'Type a rule.');
  assert.match(ruleProblem('x'.repeat(151), []) ?? '', /150/);
  assert.match(ruleProblem('a', Array(15).fill('b')) ?? '', /15/);
  assert.match(ruleProblem('Be NICE', ['be nice']) ?? '', /already/);
  assert.equal(ruleProblem('Be nice', []), null);
  assert.deepEqual(withRule(['a'], ' b '), ['a', 'b']);
  assert.deepEqual(withoutRule(['a', 'b', 'c'], 1), ['a', 'c']);
});
