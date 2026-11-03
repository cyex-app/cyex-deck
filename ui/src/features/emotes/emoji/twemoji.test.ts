import assert from 'node:assert/strict';
import { test } from 'node:test';

import { twemojiName, twemojiUrl } from './twemoji.ts';

test('plain emoji', () => {
  assert.equal(twemojiName('😀'), '1f600');
  assert.equal(twemojiName('🔥'), '1f525');
});

test('text-default emoji get the selector, whether sent or not', () => {
  assert.equal(twemojiName('❤'), '2764-fe0f');
  assert.equal(twemojiName('❤️'), '2764-fe0f');
  assert.equal(twemojiName('©'), '00a9-fe0f');
});

test('skin tones have no selector', () => {
  assert.equal(twemojiName('👍🏽'), '1f44d-1f3fd');
  assert.equal(twemojiName('☝🏻'), '261d-1f3fb');
  assert.equal(twemojiName('☝️🏻'), '261d-1f3fb');
});

test('ZWJ sequences', () => {
  assert.equal(twemojiName('👨‍👩‍👧'), '1f468-200d-1f469-200d-1f467');
  assert.equal(twemojiName('🏳️‍🌈'), '1f3f3-fe0f-200d-1f308');
  assert.equal(twemojiName('🏳‍🌈'), '1f3f3-fe0f-200d-1f308');
  assert.equal(twemojiName('🤾‍♀'), '1f93e-200d-2640-fe0f');
  assert.equal(twemojiName('👩🏽‍💻'), '1f469-1f3fd-200d-1f4bb');
});

test('keycaps', () => {
  assert.equal(twemojiName('#️⃣'), '0023-fe0f-20e3');
  assert.equal(twemojiName('1⃣'), '0031-fe0f-20e3');
  assert.equal(twemojiName('*️⃣'), '002a-fe0f-20e3');
});

test('flags', () => {
  assert.equal(twemojiName('🇩🇪'), '1f1e9-1f1ea');
});

test('url is the bundled picture and empty text has none', () => {
  assert.equal(twemojiUrl('😀'), '/twemoji/64/1f600.png');
  assert.equal(twemojiUrl(''), null);
});
