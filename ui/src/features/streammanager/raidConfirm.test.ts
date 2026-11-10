// V110-A4 Raid now and Cancel raid ask first (ConfirmButton); only Start raid is a plain button.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const src = readFileSync(new URL('./RaidCard.svelte', import.meta.url), 'utf8');

test('V110-A4 Raid now and Cancel raid go through ConfirmButton', () => {
  assert.match(src, /<ConfirmButton label="Raid now" confirm="Raid now\?"/);
  assert.equal((src.match(/<ConfirmButton label="Cancel raid" confirm="Cancel raid\?"/g) ?? []).length, 2);
  assert.doesNotMatch(src, /<Button[^>]*>\s*(Raid now|Cancel raid)\s*<\/Button>/);
});
