import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Clip } from '../../ipc/bindings/Clip.ts';
import { canRename, emptyLine, madeBy, TITLE_MAX, titleCheck } from './clips.ts';

const clip = (clippedBy: string | null): Clip => ({
  slug: 'A', title: '', views: 0, createdAt: '', duration: 0, thumbnail: null,
  channelLogin: 'cyexbot', channelName: 'cyexbot', clippedBy, game: null,
});

test('v109-c03 edit is for clips the account made', () => {
  assert.equal(madeBy(clip(null), 'mine', []), true);
  assert.equal(madeBy(clip('CyexBot'), 'channel', ['cyexbot', 'cyexbot']), true);
  assert.equal(madeBy(clip('someone'), 'channel', ['cyexbot']), false);
  assert.equal(madeBy(clip(null), 'channel', ['cyexbot']), false);
});

test('v109-c03 empty lines', () => {
  assert.equal(emptyLine('channel', true), 'No clips match that.');
  assert.match(emptyLine('mine', false), /made clips/);
  assert.match(emptyLine('channel', false), /your channel/);
});

test('v109-c05 rename where Twitch’s manager offers Edit', () => {
  const c = (channelLogin: string, editRestricted?: boolean): Clip => ({ ...clip('me'), channelLogin, editRestricted });
  // Own channel: any clip, restricted or not.
  assert.equal(canRename(c('cyexbot', true), 'channel', 'cyexbot'), true);
  // Made by me elsewhere: only while Twitch doesn’t restrict it; unknown is no.
  assert.equal(canRename(c('other', false), 'mine', 'cyexbot'), true);
  assert.equal(canRename(c('other', true), 'mine', 'cyexbot'), false);
  assert.equal(canRename(c('other'), 'mine', 'cyexbot'), false);
  // Made by me on my own channel: the owner may.
  assert.equal(canRename(c('CyexBot', true), 'mine', 'cyexbot'), true);
  assert.equal(canRename(c('other', true), 'mine', undefined), false);
});

test('v109-c05 title check', () => {
  assert.deepEqual(titleCheck('  New  ', 'Old'), { title: 'New' });
  assert.deepEqual(titleCheck(' ', 'Old'), { error: 'Add a title.' });
  assert.equal(titleCheck(' Old ', 'Old'), null);
  assert.ok('title' in (titleCheck('x'.repeat(TITLE_MAX), '') ?? {}));
  assert.match((titleCheck('x'.repeat(TITLE_MAX + 1), '') as { error: string }).error, /100/);
});
