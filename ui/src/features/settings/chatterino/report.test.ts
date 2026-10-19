import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ImportReport } from '../../../ipc/bindings/ImportReport';
import { counts, failedImport, leftOut } from './report.ts';

const empty: ImportReport = {
  tabs: 0,
  channels: 0,
  notChannels: [],
  unresolved: [],
};

test('V1041-02 counts read like the preview', () => {
  assert.equal(counts({ tabs: 1, channels: 1 }), '1 tab, 1 channel');
  assert.equal(counts({ tabs: 12, channels: 1024 }), '12 tabs, 1.024 channels');
  assert.equal(counts(empty), '0 tabs, 0 channels');
});

test('V1041-02 every reason gets a line, empty ones none', () => {
  assert.deepEqual(leftOut(empty), []);
  const lines = leftOut({
    ...empty,
    notChannels: [
      { kind: 'mentions', count: 2 },
      { kind: 'automod', count: 1 },
      { kind: 'misc', count: 1 },
      { kind: 'other', count: 2 },
    ],
    unresolved: ['gone_one', 'gone_two'],
  });
  assert.deepEqual(lines, [
    { label: 'Not channels (6)', text: 'Mentions ×2, AutoMod, Other ×3' },
    { label: 'Not found on Twitch (2)', text: 'gone_one, gone_two' },
  ]);
});

test('failedImport trims the error prefix and the full stop', () => {
  assert.equal(failedImport(new Error('no layout.')), 'Couldn’t import: no layout.');
  assert.equal(failedImport('file is empty'), 'Couldn’t import: file is empty.');
});
