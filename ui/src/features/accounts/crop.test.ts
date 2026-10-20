import assert from 'node:assert/strict';
import { test } from 'node:test';

import { clamp, drag, initial, source, zoomTo } from './crop.ts';

test('starts centered and covering', () => {
  const c = initial(800, 400, 240);
  assert.equal(c.scale, 0.6);
  assert.equal(c.y, 0);
  assert.equal(c.x, (240 - 480) / 2);
  assert.deepEqual(source(c, 240), { sx: 200, sy: 0, size: 400 });
});

test('drag never leaves a gap', () => {
  const c = initial(800, 400, 240);
  assert.equal(drag(c, 1000, 0, 800, 400, 240).x, 0);
  assert.equal(drag(c, -1000, 0, 800, 400, 240).x, 240 - 480);
  assert.equal(drag(c, 0, 50, 800, 400, 240).y, 0);
});

test('zoom keeps the center and its bounds', () => {
  const c = initial(400, 400, 240);
  const z = zoomTo(c, 2, 400, 400, 240);
  assert.equal(z.scale, 1.2);
  const s = source(z, 240);
  assert.equal(s.size, 200);
  assert.equal(s.sx + s.size / 2, 200);
  assert.equal(zoomTo(c, 99, 400, 400, 240).scale, 0.6 * 4);
  assert.equal(zoomTo(c, 0.1, 400, 400, 240).scale, 0.6);
  // Zooming out from a corner pulls the image back over the view.
  const corner = clamp({ scale: 2.4, x: 0, y: 0 }, 400, 400, 240);
  const out = zoomTo(corner, 1, 400, 400, 240);
  assert.deepEqual([out.x, out.y], [0, 0]);
  assert.deepEqual(source(out, 240), { sx: 0, sy: 0, size: 400 });
});
