import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { BASE, EASE, FAST, SLOW } from './motion.ts';

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

test('script easing matches --ease', () => {
  const m = css.match(/--ease:\s*([^;]+);/);
  assert.ok(m, '--ease in tokens.css');
  assert.equal(m[1].trim(), EASE);
});

test('V110-A-idea banners come in 4 px high over BASE ms, the pill fades over FAST ms; reduced motion is instant', async () => {
  const { bannerIn, pillIn, BASE: base, FAST: fast } = await import('./motion.ts');
  const el = {} as Element;
  const b = bannerIn(el, { reduce: false });
  assert.equal(b.duration, base);
  assert.equal(b.css(0), 'opacity:0;transform:translateY(-4px) scale(0.98)');
  assert.equal(b.css(1), 'opacity:1;transform:translateY(0px) scale(1)');
  assert.equal(pillIn(el, { reduce: false }).duration, fast);
  assert.equal(pillIn(el, { reduce: false }).css(0.5), 'opacity:0.5');
  const r = bannerIn(el, { reduce: true });
  assert.equal(r.duration, 0);
  assert.equal(r.css(0), '');
  assert.equal(pillIn(el, { reduce: true }).duration, 0);
});
