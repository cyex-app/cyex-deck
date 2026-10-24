import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { BASE, FAST, SLOW } from './motion.ts';

const css = readFileSync(new URL('../design/tokens.css', import.meta.url), 'utf8');

/** A `--name: 0.12s;` token in ms. */
function token(name: string): number {
  const m = css.match(new RegExp(`--${name}:\\s*([\\d.]+)(ms|s);`));
  assert.ok(m, `--${name} in tokens.css`);
  return Number(m[1]) * (m[2] === 's' ? 1000 : 1);
}

test('script durations match the CSS motion tokens', () => {
  assert.equal(token('fast'), FAST);
  assert.equal(token('base'), BASE);
  assert.equal(token('slow'), SLOW);
});
