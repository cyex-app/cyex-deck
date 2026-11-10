import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { BrokenFile } from '../../ipc/settings';
import { brokenRestoredText, brokenRow } from './brokenFiles.ts';

const file: BrokenFile = { name: 'tabs.broken-20261009-120000.json', partition: 'tabs', when: '2026/10/09 12:00', size: 3072 };

test('V110-A44 a broken-file row names the section, the date and the size', () => {
  assert.deepEqual(brokenRow(file), { label: 'Tabs', hint: '2026/10/09 12:00 · 3 KB' });
  assert.equal(brokenRestoredText(file), 'Restored tabs from the broken file.');
  assert.equal(brokenRow({ ...file, partition: 'streammanager' }).label, 'Stream manager');
});
