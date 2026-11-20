import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { TvPaint } from '../../ipc/bindings/TvPaint';
import { tvPaintImage, tvPaintStyle } from './tvPaint.ts';

const base: TvPaint = {
  id: 'p1',
  name: 'Sunset',
  kind: 'linear',
  angle: 90,
  repeat: false,
  shape: 'circle',
  color: '#ff0000',
  stops: [
    { at: 0, color: '#ff0000' },
    { at: 1, color: '#0000ffcc' },
  ],
  shadows: [],
  image: null,
};

test('linear and radial gradients, repeating too', () => {
  assert.equal(tvPaintImage(base), 'linear-gradient(90deg, #ff0000 0%, #0000ffcc 100%)');
  assert.equal(
    tvPaintImage({ ...base, kind: 'radial', shape: 'ellipse', repeat: true }),
    'repeating-radial-gradient(ellipse, #ff0000 0%, #0000ffcc 100%)',
  );
});

test('too few good stops fall back to the colour', () => {
  const p = { ...base, stops: [{ at: 0, color: '#ff0000' }, { at: 1, color: 'red;x' }] };
  assert.equal(tvPaintImage(p), null);
  assert.equal(tvPaintStyle(p), 'color: #ff0000');
});

test('pictures only from cdn.cyex.tv', () => {
  const p = { ...base, kind: 'image', stops: [], image: 'https://cdn.cyex.tv/cosmetics/paints/a/1x.webp' };
  assert.equal(tvPaintImage(p), 'url("https://cdn.cyex.tv/cosmetics/paints/a/1x.webp")');
  assert.equal(tvPaintImage({ ...p, image: 'https://evil.example/a.webp' }), null);
  assert.equal(tvPaintImage({ ...p, image: 'https://cdn.cyex.tv/a")b' }), null);
  assert.match(tvPaintStyle(p), /background-size: cover/);
});

test('clipped to the text, with shadows', () => {
  const s = tvPaintStyle({ ...base, shadows: [{ x: 1, y: 2, radius: 3, color: '#000000' }] });
  assert.match(s, /^color: #ff0000; background-image: linear-gradient/);
  assert.match(s, /-webkit-text-fill-color: transparent/);
  assert.match(s, /filter: drop-shadow\(1px 2px 3px #000000\)$/);
});
