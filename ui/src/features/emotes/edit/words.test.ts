import assert from 'node:assert/strict';
import { test } from 'node:test';

import { blockedText, doneText, failText, inlineText } from './words.ts';

test('every failure has a text', () => {
  const f = (type: string) => failText({ type } as never, 'kuni', 'KEKW');
  assert.match(f('signInAgain'), /sign-in ran out/);
  assert.equal(f('notAllowed'), 'You can’t edit kuni’s 7TV emotes.');
  assert.equal(f('full'), 'kuni’s 7TV emotes are full.');
  assert.equal(f('nameTaken'), 'kuni already has an emote named KEKW.');
  assert.match(f('private'), /private/);
  assert.match(f('notFound'), /isn’t there/);
  assert.match(f('badName'), /doesn’t allow/);
  assert.match(f('down'), /Nothing changed/);
  assert.equal(f('noConsent'), f('signedOut'));
  assert.match(f('testRun'), /test mock/);
  assert.match(f('bad'), /Something went wrong/);
});

test('the wait is seconds, then minutes', () => {
  const r = (seconds: number) => failText({ type: 'rateLimited', seconds }, 'c', 'a');
  assert.equal(r(12), '7TV needs a short break. Try again in 12 s.');
  assert.equal(r(90), '7TV needs a short break. Try again in 2 min.');
});

test('done texts', () => {
  assert.equal(doneText({ type: 'add', setId: 's', emoteId: 'e', alias: 'A' }, 'kuni'), 'Added A to kuni.');
  assert.equal(doneText({ type: 'remove', setId: 's', emoteId: 'e', alias: 'A' }, 'kuni'), 'Removed A from kuni.');
  assert.equal(doneText({ type: 'rename', setId: 's', emoteId: 'e', from: 'A', to: 'B' }, 'kuni'), 'Renamed A to B.');
});

test('blocked and inline texts', () => {
  assert.equal(blockedText('already', 'kuni'), 'Already there');
  assert.equal(blockedText('already'), 'Already in your channels');
  assert.equal(blockedText('private'), 'Private emote');
  assert.equal(blockedText('full', 'kuni'), 'Full');
  assert.equal(blockedText('nowhere'), 'No room in your channels');
  assert.equal(inlineText({ type: 'nameTaken' }, 'kuni'), 'Taken in kuni.');
  assert.equal(inlineText({ type: 'full' }, 'kuni'), 'kuni is full.');
  assert.equal(inlineText({ type: 'private' }, 'kuni'), 'Private emote.');
  assert.equal(inlineText({ type: 'down' }, 'kuni'), null);
});
