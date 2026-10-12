import assert from 'node:assert/strict';
import { test } from 'node:test';

import { userFontCss } from './userFonts.ts';

test('V101-10 added fonts become font faces, odd entries are skipped', () => {
  const css = userFontCss([
    { name: 'My Font', file: 'ab12cd.ttf' },
    { name: 'Bad"}', file: 'ef34.otf' },
    { name: 'Path', file: '../x.ttf' },
  ]);
  assert.equal(
    css,
    '@font-face { font-family: "My Font"; src: url("http://cyexfont.localhost/ab12cd.ttf"); font-display: swap; }',
  );
});
