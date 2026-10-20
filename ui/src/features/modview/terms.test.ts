import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { BlockedTerm } from '../../ipc/bindings/BlockedTerm';
import type { ModLine } from '../../ipc/bindings/ModLine';
import { changesTerms, shownTerms, termProblem } from './terms.ts';

const t = (text: string, createdAt: string): BlockedTerm => ({ id: text, text, createdAt, expiresAt: null });
const LIST = [t('beta', '2024-01-02T00:00:00Z'), t('Alpha', '2024-01-03T00:00:00Z'), t('gamma', '2024-01-01T00:00:00Z')];
const texts = (l: BlockedTerm[]) => l.map((x) => x.text);

test('V107-R31 newest first by default, oldest, A–Z', () => {
  assert.deepEqual(texts(shownTerms(LIST, '', 'new')), ['Alpha', 'beta', 'gamma']);
  assert.deepEqual(texts(shownTerms(LIST, '', 'old')), ['gamma', 'beta', 'Alpha']);
  assert.deepEqual(texts(shownTerms(LIST, '', 'az')), ['Alpha', 'beta', 'gamma']);
  assert.deepEqual(texts(LIST), ['beta', 'Alpha', 'gamma'], 'the list itself stays');
});

test('V107-R31 search ignores case and spaces around', () => {
  assert.deepEqual(texts(shownTerms(LIST, ' AL ', 'new')), ['Alpha']);
  assert.deepEqual(shownTerms(LIST, 'zzz', 'new'), []);
});

test('V107-R31 add check: too short, too long, already there', () => {
  assert.match(termProblem(' a ', LIST) ?? '', /at least 2/);
  assert.match(termProblem('x'.repeat(501), LIST) ?? '', /up to 500/);
  assert.equal(termProblem('x'.repeat(500), LIST), null);
  assert.match(termProblem('ALPHA', LIST) ?? '', /already/);
  assert.equal(termProblem('delta', LIST), null);
});

test('V107-R31 live blocked/unblocked actions refresh the list, history doesn’t', () => {
  const line = (action: string, past = false) => ({ action, past }) as ModLine;
  assert.ok(changesTerms(line('blocked')) && changesTerms(line('unblocked')));
  assert.ok(!changesTerms(line('blocked', true)) && !changesTerms(line('banned')));
});
