import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Clip } from '../../ipc/bindings/Clip.ts';
import { emptyLine, madeBy } from './clips.ts';

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
