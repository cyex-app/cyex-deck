import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { PaintStyle } from '../../ipc/bindings/PaintStyle.ts';
import { loadAll, paintUrls } from './paintPics.ts';

const U = 'https://cdn.7tv.app/paint/P/layer/L/1x.webp';
const S = 'https://cdn.7tv.app/paint/P/layer/L/1x_static.webp';
const style = (css: string, still: string | null = null): PaintStyle => ({ id: 'P', name: 'p', css, still });

test('1.37 a picture paint lists its moving and still pictures', () => {
  const p = style(`background-image:url("${U}");--paint-shadow:drop-shadow(0px 0px 1px rgba(0,0,0,1));`, `background-image:url("${S}");`);
  assert.deepEqual(paintUrls(p), [U, S]);
  assert.deepEqual(paintUrls(style('background-image:linear-gradient(90deg,rgba(255,0,0,1) 0%);')), []);
});

test('1.37 a picture that failed once is asked again until it loads', async () => {
  const asked: string[] = [];
  const waits: number[] = [];
  const load = async (u: string) => (asked.push(u), u !== S || asked.filter((a) => a === S).length > 1);
  const ok = await loadAll([U, S], load, async (ms) => void waits.push(ms), [5, 10]);
  assert.equal(ok, true);
  assert.deepEqual(asked, [U, S, S]);
  assert.deepEqual(waits, [5]);
});

test('1.37 a picture that keeps failing stops after the last wait', async () => {
  let n = 0;
  const ok = await loadAll([U], async () => (n++, false), async () => {}, [1, 1]);
  assert.equal(ok, false);
  assert.equal(n, 3);
});
