import assert from 'node:assert/strict';
import { test } from 'node:test';

import { APP_FONT, FONTS, fontName, fontValue } from './fonts.ts';

test('the eight fonts, each once', () => {
  assert.equal(FONTS.length, 8);
  assert.equal(new Set(FONTS).size, 8);
  assert.ok(FONTS.includes(APP_FONT));
});

test('the app font is stored as empty and read back', () => {
  assert.equal(fontValue('IBM Plex Sans'), '');
  assert.equal(fontValue('Lato'), 'Lato');
  assert.equal(fontName(''), 'IBM Plex Sans');
  assert.equal(fontName('Arial'), 'Arial');
});
