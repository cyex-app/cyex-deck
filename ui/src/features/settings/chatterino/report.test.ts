import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ImportReport } from '../../../ipc/bindings/ImportReport';
import { anonymousNote, anonymousOffer, counts, leftOut } from './report.ts';

const empty: ImportReport = {
  tabs: 0,
  channels: 0,
  anonymous: 0,
  alreadyOpen: [],
  notChannels: [],
  unresolved: [],
  overLimit: [],
};

test('X12 the offer names how many', () => {
  assert.equal(anonymousOffer(1), 'Add the 1 channel past the limit as an anonymous tab');
  assert.equal(anonymousOffer(1024), 'Add the 1.024 channels past the limit as anonymous tabs');
});

test('X12 the result says how many joined anonymously', () => {
  assert.equal(anonymousNote(empty), null);
  assert.equal(anonymousNote({ anonymous: 1 }), '1 channel past the limit joined anonymously and can only read chat.');
  assert.equal(anonymousNote({ anonymous: 1024 }), '1.024 channels past the limit joined anonymously and can only read chat.');
});

test('V1041-02 counts read like the preview', () => {
  assert.equal(counts({ tabs: 1, channels: 1 }), '1 tab, 1 channel');
  assert.equal(counts({ tabs: 12, channels: 1024 }), '12 tabs, 1.024 channels');
  assert.equal(counts(empty), '0 tabs, 0 channels');
});

test('V1041-02 every reason gets a line, empty ones none', () => {
  assert.deepEqual(leftOut(empty), []);
  const lines = leftOut({
    ...empty,
    alreadyOpen: ['alpha_one'],
    notChannels: [
      { kind: 'mentions', count: 2 },
      { kind: 'automod', count: 1 },
      { kind: 'misc', count: 1 },
      { kind: 'other', count: 2 },
    ],
    unresolved: ['gone_one', 'gone_two'],
    overLimit: ['chan_101'],
  });
  assert.deepEqual(lines, [
    { label: 'Already open (1)', text: 'alpha_one' },
    { label: 'Not channels (6)', text: 'Mentions ×2, AutoMod, Other ×3' },
    { label: 'Not found on Twitch (2)', text: 'gone_one, gone_two' },
    { label: 'Past the 100-chat limit (1)', text: 'chan_101' },
  ]);
});
