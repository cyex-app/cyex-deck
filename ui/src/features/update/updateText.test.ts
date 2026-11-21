import assert from 'node:assert/strict';
import { test } from 'node:test';

import { checkText, displayVersion, progressText, releaseNotesUrl, releaseTag } from './updateText.ts';

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

test('release tags pad the patch', () => {
  assert.equal(releaseTag('0.1.7'), 'v0.1.07');
  assert.equal(releaseTag('0.1.10'), 'v0.1.10');
  assert.equal(releaseTag('v0.1.9-beta.2'), 'v0.1.09-beta.2');
  assert.equal(displayVersion('0.1.7'), '0.1.07');
  assert.equal(releaseNotesUrl('0.1.10'), 'https://github.com/cyex-app/cyex-deck/releases/tag/v0.1.10');
});
