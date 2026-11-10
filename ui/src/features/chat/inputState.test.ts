import assert from 'node:assert/strict';
import { test } from 'node:test';

import { clearDrafts, DRAFT_CHARS, pasteCut, DRAFTS_KEY, persistDrafts, resetDrafts, draftKey, dropLoneHigh, loadDraft, saveDraft, syncRecall, upRecalls } from './inputState.ts';

const sent = ['b', 'a'];

test('V110-F86 typing over a recalled line ends the recall', () => {
  assert.equal(syncRecall(0, 'b', sent), 0);
  assert.equal(syncRecall(0, 'a brand new message', sent), -1);
  assert.equal(syncRecall(-1, 'b', sent), -1);
  assert.equal(upRecalls('a brand new message', 0, sent), false);
  assert.equal(upRecalls('b', 0, sent), true);
  assert.equal(upRecalls('', -1, sent), true);
  assert.equal(upRecalls('typed', -1, sent), false);
  assert.equal(upRecalls('', -1, []), false);
});

test('V110-F86 a cut inside an emoji drops the lone half', () => {
  const e = '\u{1F600}';
  assert.equal(dropLoneHigh(`ab${e.slice(0, 1)}`), 'ab');
  assert.equal(dropLoneHigh(`ab${e}`), `ab${e}`);
  assert.equal(dropLoneHigh(''), '');
});

test('V110-F86 draft and sent lines are kept per chat, capped', () => {
  clearDrafts();
  const k = draftKey('Me', 'Chan');
  assert.equal(k, 'Me:chan');
  assert.deepEqual(loadDraft(k), { text: '', sent: [] });
  saveDraft(k, 'half a thought', ['x', 'y']);
  const d = loadDraft(k);
  assert.deepEqual(d, { text: 'half a thought', sent: ['x', 'y'] });
  d.sent.push('z');
  assert.deepEqual(loadDraft(k).sent, ['x', 'y']);
  saveDraft(k, '', Array.from({ length: 80 }, (_, i) => `l${i}`));
  assert.equal(loadDraft(k).sent.length, 50);
  saveDraft(k, '', []);
  assert.deepEqual(loadDraft(k), { text: '', sent: [] });
  for (let i = 0; i < 60; i++) saveDraft(`a:c${i}`, 't', []);
  assert.equal(loadDraft('a:c0').text, '');
  assert.equal(loadDraft('a:c59').text, 't');
});

const memory = () => {
  const m = new Map<string, string>();
  return { m, store: { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v), removeItem: (k: string) => void m.delete(k) } };
};

test('V110-A-idea drafts come back after a restart: text only, capped', () => {
  const { m, store } = memory();
  resetDrafts(store);
  saveDraft('a:one', 'half a thought', ['sent line']);
  saveDraft('a:two', 'x'.repeat(DRAFT_CHARS + 50), []);
  saveDraft('a:three', '', ['only history']);
  persistDrafts();
  const saved = JSON.parse(m.get(DRAFTS_KEY)!) as [string, string][];
  assert.deepEqual(saved.map((e) => e[0]), ['a:one', 'a:two'], 'no entry without text');
  assert.equal(saved[1][1].length, DRAFT_CHARS);
  resetDrafts(store);
  assert.deepEqual(loadDraft('a:one'), { text: 'half a thought', sent: [] }, 'sent lines are not kept');
  assert.equal(loadDraft('a:two').text.length, DRAFT_CHARS);
  assert.equal(loadDraft('a:three').text, '');
});

test('V110-A-idea a draft typed before the restore wins; a cleared draft leaves no key; bad data is ignored', () => {
  const { m, store } = memory();
  m.set(DRAFTS_KEY, JSON.stringify([['a:one', 'old'], ['a:two', 'kept'], [3, 'x'], ['a:bad', 5]]));
  resetDrafts(store);
  saveDraft('a:one', 'new', []);
  assert.equal(loadDraft('a:one').text, 'new');
  assert.equal(loadDraft('a:two').text, 'kept');
  assert.equal(loadDraft('a:bad').text, '');
  saveDraft('a:one', '', []);
  saveDraft('a:two', '', []);
  persistDrafts();
  assert.equal(m.has(DRAFTS_KEY), false);
  m.set(DRAFTS_KEY, '{not json');
  resetDrafts(store);
  assert.equal(loadDraft('a:one').text, '');
  resetDrafts(null);
  assert.doesNotThrow(() => persistDrafts());
});

test('V110-A-idea more than 50 saved chats keep the newest 50', () => {
  const { m, store } = memory();
  m.set(DRAFTS_KEY, JSON.stringify(Array.from({ length: 70 }, (_, i) => [`a:c${i}`, 't'])));
  resetDrafts(store);
  assert.equal(loadDraft('a:c19').text, '');
  assert.equal(loadDraft('a:c20').text, 't');
  assert.equal(loadDraft('a:c69').text, 't');
});

test('V110-A-idea a paste past the box limit is a cut; the selection it replaces gives room back', () => {
  assert.equal(pasteCut(0, 0, 'a'.repeat(1000), 1000), false);
  assert.equal(pasteCut(0, 0, 'a'.repeat(1001), 1000), true);
  assert.equal(pasteCut(900, 0, 'a'.repeat(101), 1000), true);
  assert.equal(pasteCut(900, 50, 'a'.repeat(150), 1000), false);
  assert.equal(pasteCut(0, 0, 'a\r\n'.repeat(500), 1000), false, 'CR LF is one character in the box');
});
