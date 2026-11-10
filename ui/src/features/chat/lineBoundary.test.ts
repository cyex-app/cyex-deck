// V110-A-idea A chat line that throws while drawing must not take the list down.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const src = readFileSync(new URL('./MessageList.svelte', import.meta.url), 'utf8');

test('V110-A-idea every line of the chat list sits in a boundary with a plain fallback', () => {
  assert.match(src, /<svelte:boundary>\s*<Line [^>]*\/>\s*\{#snippet failed\(\)\}[^]*?Couldn’t show this line[^]*?<\/svelte:boundary>/);
});
