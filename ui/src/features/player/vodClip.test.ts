import assert from 'node:assert/strict';
import { test } from 'node:test';

import { ownVideo, vodClipOffset } from './vodClip.ts';

test('the clip ends at the play head, inside the video', () => {
  assert.equal(vodClipOffset(5025.7, 8000), 5025);
  assert.equal(vodClipOffset(9000, 8000), 8000);
  assert.equal(vodClipOffset(-3, 8000), 0);
  assert.equal(vodClipOffset(NaN, 8000), 0);
  assert.equal(vodClipOffset(12, 0), 12);
});

test('only the tab’s own channel', () => {
  assert.equal(ownVideo('XQC', 'xqc'), true);
  assert.equal(ownVideo('xqc', 'pokimane'), false);
  assert.equal(ownVideo(undefined, 'xqc'), false);
  assert.equal(ownVideo('xqc', undefined), false);
});
