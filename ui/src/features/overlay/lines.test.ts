import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { overlayShows } from './lines.ts';

const n = (kind: string) => ({ notice: { kind } }) as never;

test('V110-B4 the overlay hides the app\'s own info and error lines, by kind', () => {
  assert.equal(overlayShows({}), true);
  for (const k of ['sub', 'gift', 'raid', 'announcement', 'bits', 'milestone', 'redemption', 'moderation']) {
    assert.equal(overlayShows(n(k)), true, k);
  }
  assert.equal(overlayShows(n('info')), false);
  assert.equal(overlayShows(n('error')), false);
  const src = readFileSync(new URL('./OverlayView.svelte', import.meta.url), 'utf8');
  assert.match(src, /overlayShows\(m\)/);
});
