import assert from 'node:assert/strict';
import { test } from 'node:test';

import { adjustedNote, appliedHex, isLink, keptNote, parseHex } from './themes.ts';

test('nudged colors read as a sentence', () => {
  assert.equal(adjustedNote([]), '');
  assert.equal(adjustedNote(['text']), 'Text was nudged so everything stays readable.');
  assert.equal(
    adjustedNote(['bg', 'text', 'dim']),
    'Background, Text and Faint text were nudged so everything stays readable.',
  );
});

test('wallpaper links match what Rust takes', () => {
  assert.ok(isLink('https://example.com/a.png'));
  assert.ok(!isLink('http://example.com/a.png'));
  assert.ok(!isLink('https://example.com/a b.png'));
  assert.ok(!isLink('https://example.com/a.png") x'));
  assert.ok(!isLink('abc123.png'));
});

test('T2 hex colors are tidied', () => {
  assert.equal(parseHex('#F0F0F0'), '#f0f0f0');
  assert.equal(parseHex(' abc '), '#aabbcc');
  assert.equal(parseHex('#12345'), null);
  assert.equal(parseHex('red'), null);
});

test('T1 the painted color and why it differs', () => {
  assert.equal(appliedHex('#1E1F23'), '#1e1f23');
  assert.equal(appliedHex('rgb(1, 2, 3)'), null);
  assert.equal(appliedHex(undefined), null);
  assert.equal(keptNote('bg', 'dark'), 'Kept dark so text stays readable');
  assert.equal(keptNote('text', 'light'), 'Nudged so it stays readable');
});
