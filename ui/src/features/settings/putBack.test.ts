import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { putBackText } from './putBack.ts';

test('V110-N5 R2-3 the automations note only when they were on and are off now', () => {
  assert.equal(putBackText(false, false), 'Put back.');
  assert.equal(putBackText(true, true), 'Put back.');
  assert.equal(putBackText(false, true), 'Put back.');
  assert.match(putBackText(true, false), /Automations are off/);
});

test('V110-N5 R2-3 undo uses it', () => {
  const src = readFileSync(new URL('./sections/DataSection.svelte', import.meta.url), 'utf8');
  assert.match(src, /putBackText\(on, settings\.v\.commands\.automationsOn\)/);
});
