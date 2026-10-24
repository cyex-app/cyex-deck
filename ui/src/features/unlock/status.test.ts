import assert from 'node:assert/strict';
import { test } from 'node:test';

import { doneLead, sevenTvNote, statusOf } from './status.ts';

test('7TV says where its window is', () => {
  assert.equal(statusOf('seventv', 'running').lead, 'Opening the 7TV window…');
  assert.equal(statusOf('seventv', 'running', { stage: 'window' }).lead, 'Finish signing in in the 7TV window.');
  assert.equal(statusOf('seventv', 'running', { stage: 'hidden' }).lead, 'Connecting to 7TV…');
  assert.equal(statusOf('seventv', 'running', { stage: 'connecting' }).detail, 'Checking it’s your Twitch account.');
  assert.equal(statusOf('seventv', 'done').lead, '7TV is connected.');
});

test('a failure says why, else that the window closed', () => {
  assert.equal(statusOf('seventv', 'failed').detail, 'The 7TV window closed before it finished.');
  assert.equal(statusOf('seventv', 'failed', { note: 'Nope.' }).detail, 'Nope.');
  assert.equal(statusOf('twitch', 'failed').detail, 'The sign-in didn’t finish.');
});

test('Twitch shows its code while it waits', () => {
  assert.equal(statusOf('twitch', 'running', { code: 'ABCD1234' }).lead, 'Approve ABCD1234 in the Twitch window.');
  assert.equal(statusOf('twitch', 'running', { opened: true }).lead, 'Finish signing in on Twitch.');
  assert.equal(statusOf('twitch', 'running').lead, 'Opening Twitch…');
});

test('7TV’s answer becomes the note', () => {
  assert.equal(sevenTvNote({ type: 'done' }, 'Nora'), null);
  assert.equal(sevenTvNote({ type: 'otherAccount' }, 'Nora'), 'That’s a different Twitch account than Nora.');
  assert.equal(sevenTvNote({ type: 'cancelled' }, 'Nora'), '');
  assert.match(sevenTvNote({ type: 'failed' }, 'Nora') ?? '', /^Couldn’t reach 7TV/);
});

test('the end line', () => {
  assert.equal(doneLead(['seventv']), '7TV is connected.');
  assert.equal(doneLead(['twitch', 'seventv']), 'All set.');
});
