import assert from 'node:assert/strict';
import { test } from 'node:test';

import { clearDrafts, draftKey, dropLoneHigh, loadDraft, saveDraft, syncRecall, upRecalls } from './inputState.ts';

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
