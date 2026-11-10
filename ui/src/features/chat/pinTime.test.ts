import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { pinSent } from './pinTime.ts';

const at = (y: number, mo: number, d: number, h: number, mi: number) => new Date(y, mo - 1, d, h, mi).getTime();

test('V110-B5 a pin sent today shows its time, an older one its date too, an unknown one nothing', () => {
  const now = at(2026, 11, 10, 18, 30);
  assert.equal(pinSent(at(2026, 11, 10, 14, 3), now), '14:03');
  assert.equal(pinSent(at(2026, 11, 8, 9, 5), now), '2026/11/08 09:05');
  assert.equal(pinSent(0, now), '');
  assert.equal(pinSent(Number.NaN, now), '');
});

test('V110-B5 the banner draws the send time', () => {
  const src = readFileSync(new URL('./PinnedBanner.svelte', import.meta.url), 'utf8');
  assert.match(src, /pinSent\(pin\.message\.ts/);
});
