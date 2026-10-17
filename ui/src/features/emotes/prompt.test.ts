import assert from 'node:assert/strict';
import { test } from 'node:test';

import { unlistedPrompt } from './prompt.ts';

test('unlisted prompt shows only for blur, never chosen, unlisted (V106-30)', () => {
  const c = (unlistedEmotes: 'hide' | 'blur' | 'show', unlistedChosen: boolean) => ({ unlistedEmotes, unlistedChosen });
  assert.equal(unlistedPrompt(c('blur', false), false), false);
  assert.equal(unlistedPrompt(c('hide', false), true), false);
  assert.equal(unlistedPrompt(c('show', false), true), false);
  assert.equal(unlistedPrompt(c('blur', true), true), false);
  assert.equal(unlistedPrompt(c('blur', false), true), true);
});
