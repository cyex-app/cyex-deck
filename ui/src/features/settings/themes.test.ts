import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Appearance } from '../../ipc/bindings/Appearance';
import type { CustomTheme } from '../../ipc/bindings/CustomTheme';
import {
  accentOf,
  adjustedNote,
  appliedHex,
  choiceOf,
  defaultAccentOf,
  freeThemeName,
  isLink,
  keptNote,
  newThemeFrom,
  parseHex,
} from './themes.ts';

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

const wall = { source: 'w.png', area: 'window', dim: 50, blur: 2, fit: 'cover' } as const;
const dusk: CustomTheme = { id: 'dusk', name: 'Dusk', base: 'dark', colors: { bg: '#18141c' }, accent: '#d4679f', wallpaper: { ...wall, source: '' } };
const neon: CustomTheme = { id: 'ab12cd34ef', name: 'Neon', base: 'light', colors: { bg: '#fff' }, accent: '#00ff00', wallpaper: wall };
const app = (over: Partial<Appearance> = {}): Appearance =>
  ({ theme: 'dark', accent: '#145aff', themes: [neon], themeId: '', accents: {}, ...over }) as Appearance;
const builtins = [dusk];

test('the theme in use is found by id, custom first', () => {
  assert.deepEqual(choiceOf(app(), builtins), { kind: 'plain', base: 'dark' });
  assert.deepEqual(choiceOf(app({ themeId: 'dusk' }), builtins), { kind: 'builtin', theme: dusk });
  assert.deepEqual(choiceOf(app({ themeId: neon.id }), builtins), { kind: 'custom', theme: neon });
  assert.deepEqual(choiceOf(app({ themeId: 'gone', theme: 'light' }), builtins), { kind: 'plain', base: 'light' });
  const clash = { ...neon, id: 'dusk' };
  assert.equal(choiceOf(app({ themeId: 'dusk', themes: [clash] }), builtins).kind, 'custom');
});

test('accent in use and the one Default returns to', () => {
  const b = choiceOf(app({ themeId: 'dusk' }), builtins);
  assert.equal(accentOf(app({ themeId: 'dusk' }), b), '#d4679f');
  assert.equal(accentOf(app({ themeId: 'dusk', accents: { dusk: '#123456' } }), b), '#123456');
  assert.equal(accentOf(app(), { kind: 'custom', theme: neon }), '#00ff00');
  assert.equal(accentOf(app({ accent: '#abcdef' }), { kind: 'plain', base: 'dark' }), '#abcdef');
  assert.equal(defaultAccentOf(b), '#d4679f');
  assert.equal(defaultAccentOf({ kind: 'custom', theme: neon }), '#145aff');
  assert.equal(defaultAccentOf({ kind: 'plain', base: 'light' }), '#145aff');
});

test('a new theme copies what shows now', () => {
  const plain = newThemeFrom(app({ theme: 'light' }), { kind: 'plain', base: 'light' });
  assert.equal(plain.name, 'My theme');
  assert.equal(plain.base, 'light');
  assert.equal(plain.id, '');
  const a = app({ themeId: 'dusk', accents: { dusk: '#123456' } });
  const fromBuiltin = newThemeFrom(a, choiceOf(a, builtins));
  assert.equal(fromBuiltin.name, 'My theme');
  assert.equal(fromBuiltin.accent, '#123456');
  assert.equal(fromBuiltin.wallpaper.source, '');
  const fromCustom = newThemeFrom(app({ themes: [neon] }), { kind: 'custom', theme: neon });
  assert.equal(fromCustom.name, 'My theme');
  assert.equal(fromCustom.wallpaper.source, 'w.png');
  assert.notEqual(fromCustom.colors, neon.colors);
});

test('new theme names take the first free number, never "copy copy"', () => {
  const named = (...names: string[]) => names.map((name) => ({ ...neon, name }));
  assert.equal(freeThemeName([]), 'My theme');
  assert.equal(freeThemeName(named('My theme')), 'My theme 2');
  assert.equal(freeThemeName(named('My theme', 'My theme 2', 'my theme 4')), 'My theme 3');
  assert.equal(freeThemeName(named('My theme 2')), 'My theme');
});
