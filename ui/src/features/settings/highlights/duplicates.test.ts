import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { PhraseRule } from '../../../ipc/bindings/PhraseRule';
import { groups, leadText, removeCopies } from './duplicates.ts';

const rule = (pattern: string, over: Partial<PhraseRule> = {}): PhraseRule =>
  ({ enabled: true, pattern, regex: false, flavor: 'rust', caseSensitive: false, wholeWord: true, lookalikes: false, color: '', sound: false, flash: false, show: true, ...over }) as PhraseRule;

const list = [rule('gg'), rule('hi'), rule('GG', { sound: true }), rule('yo'), rule('gg ')];
const checks = [{}, {}, { sameAs: 1 }, {}, { sameAs: 1 }];

test('groups put the copies under the first row (V106-33)', () => {
  assert.deepEqual(groups(list, checks), [{ first: 0, copies: [2, 4] }]);
  assert.deepEqual(groups(list, []), []);
});

test('groups ignore a sameAs that points forward or out of range', () => {
  assert.deepEqual(groups(list, [{ sameAs: 3 }, {}, {}, { sameAs: 99 }, {}]), []);
});

test('groups are listed by their first row', () => {
  const g = groups([rule('a'), rule('b'), rule('b'), rule('a')], [{}, {}, { sameAs: 2 }, { sameAs: 1 }]);
  assert.deepEqual(g, [
    { first: 0, copies: [3] },
    { first: 1, copies: [2] },
  ]);
});

test('removeCopies deletes the later copies and keeps the first as it was (O3)', () => {
  const out = removeCopies(list, checks);
  assert.deepEqual(out.map((r) => r.pattern), ['gg', 'hi', 'yo']);
  assert.equal(out[0], list[0]);
  assert.equal(out[0].sound, false);
});

test('removeCopies with no duplicates returns the same rows', () => {
  assert.deepEqual(removeCopies(list, []), list);
});

test('leadText names where the copies are (b-12)', () => {
  assert.match(leadText(true, false), /in Words more than once/);
  assert.match(leadText(false, true), /in Global mentions more than once/);
  assert.match(leadText(true, true), /in Words and in Global mentions/);
  assert.doesNotMatch(leadText(true, false), /both/);
});
