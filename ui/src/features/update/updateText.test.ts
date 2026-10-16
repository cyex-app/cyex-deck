import assert from 'node:assert/strict';
import { test } from 'node:test';

import { checkText, progressText } from './updateText.ts';

test('check results read short', () => {
  assert.equal(checkText({ kind: 'newest' }), 'You have the newest version.');
  assert.equal(checkText({ kind: 'newest' }, '0.1.5'), 'You have the newest version, 0.1.5.');
  assert.equal(
    checkText({
      kind: 'available',
      update: { version: '0.1.6', prerelease: false, notesUrl: '', canInstall: true },
    }),
    '0.1.6 is out.',
  );
  assert.equal(checkText({ kind: 'failed' }), "Couldn’t check. Try again later.");
});

test('progress reads short', () => {
  assert.equal(progressText({ kind: 'downloading', percent: 42 }), 'Downloading… 42 %');
  assert.equal(progressText({ kind: 'verifying' }), 'Checking the file…');
  assert.equal(progressText({ kind: 'restarting' }), 'Restarting…');
});
