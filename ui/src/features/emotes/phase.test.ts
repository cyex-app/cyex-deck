import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { IN_OUT, MOD_CYCLE_MS, MOTION, MOVING, SLIDE_SECS, motion, moves, slideSecs, wallStart } from './phase.ts';

test('the start time follows the wall clock', () => {
  assert.equal(wallStart(1_700_000_012_345), -(1_700_000_012_345 % 6000));
});

test('every modifier period divides the cycle, and every animation has its keyframes', () => {
  const css = readFileSync(new URL('../../design/emotes.css', import.meta.url), 'utf8');
  const slides = SLIDE_SECS.map((s) => `emote-slide ${s}s linear infinite`);
  for (const a of [...Object.values(MOTION), IN_OUT, ...slides]) {
    const [, name, secs] = /^(emote-[\w-]+) ([\d.]+)s /.exec(a) ?? [];
    assert.ok(name, a);
    assert.equal(MOD_CYCLE_MS % Math.round(Number(secs) * 1000), 0, `${a} doesn't divide the cycle`);
    assert.ok(css.includes(`@keyframes ${name} {`), `${name} has no keyframes`);
  }
  assert.ok(css.includes('@keyframes emote-slide-2 {'));
});

test('which classes move', () => {
  assert.equal(moves('m-flipX m-rotate'), true);
  assert.equal(moves('m-flipX m-party'), true);
  assert.equal(moves('m-flipX'), false);
  assert.equal(moves(''), false);
  assert.deepEqual([...MOVING].sort(), Object.keys(MOTION).sort());
});

test('V111-20 moving effects run together, as FFZ combines them', () => {
  assert.equal(motion([]), '');
  assert.equal(motion(['shake', 'rainbow']), `${MOTION.rainbow}, ${MOTION.shake}`);
  // Appear and leave together come in and go out; nothing spins while it slides.
  assert.equal(motion(['appear', 'leave']), IN_OUT);
  assert.equal(motion(['rotate', 'slide'], 2), 'emote-slide 2s linear infinite');
});

test('V111-20 a slide takes half a second per square, on the cycle', () => {
  assert.equal(slideSecs(1), 0.5);
  assert.equal(slideSecs(0.5), 0.5);
  assert.equal(slideSecs(2), 1);
  assert.equal(slideSecs(2.2), 1.2);
  assert.equal(slideSecs(4), 2);
  assert.equal(slideSecs(40), 3);
});
