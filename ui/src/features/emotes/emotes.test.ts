import assert from 'node:assert/strict';
import { test } from 'node:test';

import { MOD, emoteRatio, modClasses, modLook } from './emotes.ts';

test('modifier bits become classes', () => {
  assert.equal(modClasses(undefined), '');
  assert.equal(modClasses(MOD.flipX | MOD.rainbow), 'm-flipX m-rainbow');
  assert.equal(modClasses(MOD.hidden), '');
});

test('V111-20 BTTV and FFZ looks', () => {
  assert.deepEqual(modLook(undefined, null), { classes: '', ratio: null, animation: '', slide: false });
  // ffzHyper: red and the fast shake, the hidden bit draws nothing.
  const hyper = modLook(MOD.hidden | MOD.hyperRed | MOD.shake, [28, 28]);
  assert.equal(hyper.classes, 'm-hyperRed m-shake');
  assert.equal(hyper.animation, 'emote-shake 0.1s linear infinite');
  // BTTV w! is 4:1 whatever the emote; FFZ GrowX doubles its own width, at most 4:1.
  assert.equal(modLook(MOD.wide, [28, 28]).ratio, 4);
  assert.equal(modLook(MOD.growX, [28, 28]).ratio, 2);
  assert.equal(modLook(MOD.growX, [84, 28]).ratio, 4);
  assert.equal(modLook(MOD.growX, null).ratio, 2);
  assert.equal(modLook(MOD.flipX, [28, 28]).ratio, null);
  // BTTV's own: s! jitters in steps, p! cycles colours.
  assert.equal(modLook(MOD.jitter | MOD.party, [28, 28]).animation, 'emote-party 1.5s linear infinite, emote-jitter 0.5s step-start infinite');
  // FFZ Slide: twice as long when it's twice as wide.
  const slide = modLook(MOD.slide | MOD.growX, [28, 28]);
  assert.equal(slide.slide, true);
  assert.equal(slide.animation, 'emote-slide 1s linear infinite');
  assert.equal(modClasses(MOD.wide | MOD.rotateLeft), 'm-wide m-rotateLeft');
});

test('emotes keep their own shape (V102-06)', () => {
  assert.equal(emoteRatio(undefined), 'auto 1 / 1');
  assert.equal(emoteRatio([112, 32]), 'auto 112 / 32');
});
