import assert from 'node:assert/strict';
import { test } from 'node:test';

import { adjustedNote, isLink } from './themes.ts';

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
