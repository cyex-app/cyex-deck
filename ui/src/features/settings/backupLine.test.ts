import assert from 'node:assert/strict';
import { test } from 'node:test';

import { backupText } from './backupLine.ts';

const when = (d: Date) => `T${d.getTime()}`;

test('V110-U55 a good copy reads as a time, a failure says why and that it retries', () => {
  assert.equal(backupText(null, when), null);
  assert.equal(backupText({ lastOk: null, error: null }, when), null);
  assert.deepEqual(backupText({ lastOk: 100, error: null }, when), { text: 'Last backup T100000', failing: false });
  assert.deepEqual(backupText({ lastOk: 100, error: 'the disk is full.' }, when), {
    text: 'Backups are failing: the disk is full. Trying again every hour.',
    failing: true,
  });
});
