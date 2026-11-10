// V110-A35 The event lines (sub, gift, announcement…) take their colour from the highlight settings (--hl).
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const css = readFileSync(new URL('./Line.svelte', import.meta.url), 'utf8');

test('V110-A35 the event line border follows --hl, with the Twitch purple as the fallback', () => {
  const rule = css.match(/\.kind-announcement,\s*\n\s*\.kind-bits,[^{]*\{([^}]*)\}/);
  assert.ok(rule, 'the shared event-line rule');
  assert.match(rule[1], /border-left:\s*3px solid var\(--hl, var\(--twitch\)\)/);
  assert.match(rule[1], /var\(--hl, var\(--twitch\)\) 7%/);
});

test('V110-A35 a coloured event line has no second inset rail', () => {
  assert.match(css, /\.hl\.kind-announcement,[^{]*\{[^}]*box-shadow:\s*none/);
});
