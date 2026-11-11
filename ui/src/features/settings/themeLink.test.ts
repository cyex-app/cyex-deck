import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { CustomTheme } from '../../ipc/bindings/CustomTheme';
import type { ThemeLink } from '../../ipc/bindings/ThemeLink';
import { asksBeforeUpload, copyLabel, linkOf, shareState, updateAvailable, updateLabel, withLink } from './themeLink.ts';

const blank: ThemeLink = {
  sharedId: '',
  sharedVersion: 0,
  sharedHash: '',
  sharedWallpaper: '',
  sharedChanged: false,
  sourceId: '',
  sourceVersion: 0,
  sourceDate: '',
  sourceHash: '',
  latestVersion: 0,
  ignoredVersion: 0,
  removed: false,
  edited: false,
  history: [],
};

const theme = (link?: Partial<ThemeLink>): CustomTheme => ({
  id: 'a1',
  name: 'Neon',
  base: 'dark',
  colors: {},
  accent: '#145aff',
  wallpaper: { source: '', area: 'window', dim: 70, blur: 0, fit: 'cover' },
  ...(link ? { link: { ...blank, ...link } } : {}),
});

test('V109-T01 update notice', () => {
  assert.equal(updateAvailable(undefined), null);
  const from = { sourceId: 'k3f9x2ab', sourceVersion: 2 };
  assert.equal(updateAvailable({ ...blank, ...from, latestVersion: 2 }), null);
  assert.equal(updateAvailable({ ...blank, ...from, latestVersion: 4 }), 4);
  assert.equal(updateAvailable({ ...blank, ...from, latestVersion: 4, ignoredVersion: 4 }), null, 'ignored');
  assert.equal(updateAvailable({ ...blank, ...from, latestVersion: 5, ignoredVersion: 4 }), 5, 'a newer one shows again');
  assert.equal(updateAvailable({ ...blank, ...from, latestVersion: 4, removed: true }), null, 'removed');
  assert.equal(updateAvailable({ ...blank, latestVersion: 4 }), null, 'not from a link');
  assert.equal(updateLabel(4), 'Update available · v4');
});

test('V109-T01 go back labels', () => {
  const t = theme();
  assert.equal(copyLabel({ version: 3, date: '2026/11/02', edited: false, theme: t }), 'v3 · 2026/11/02');
  assert.equal(copyLabel({ version: 3, date: '', edited: true, theme: t }), 'v3 · your changes');
});

test('V109-T01 share state', () => {
  assert.deepEqual(shareState(theme()), { kind: 'none' });
  assert.deepEqual(shareState(theme({ sourceId: 'k3f9x2ab' })), { kind: 'none' }, 'someone else’s link');
  assert.deepEqual(shareState(theme({ sharedId: 'k3f9x2ab', sharedVersion: 2, sharedChanged: true })), {
    kind: 'shared',
    id: 'k3f9x2ab',
    link: 'https://cyex.app/t/k3f9x2ab',
    version: 2,
    changed: true,
  });
  assert.equal(linkOf('m4n5p6q7'), 'https://cyex.app/t/m4n5p6q7');
});

test('V109-T01 editor saves keep the saved link', () => {
  const saved = [theme({ sourceId: 'k3f9x2ab', latestVersion: 4 })];
  const stale = { ...theme({ sourceId: 'k3f9x2ab', latestVersion: 1 }), accent: '#ff0000' };
  const out = withLink(stale, saved);
  assert.equal(out.accent, '#ff0000');
  assert.equal(out.link?.latestVersion, 4);
  assert.equal(withLink(stale, []).link, undefined, 'a theme without a saved link has none');
});

test('Copy code asks only before a local wallpaper goes up', () => {
  const wp = (source: string, link?: Partial<ThemeLink>): CustomTheme => ({
    ...theme(link),
    wallpaper: { source, area: 'window', dim: 70, blur: 0, fit: 'cover' },
  });
  assert.equal(asksBeforeUpload(wp('')), false, 'no wallpaper');
  assert.equal(asksBeforeUpload(wp('https://example.com/a.png')), false, 'a link');
  assert.equal(asksBeforeUpload(wp('abc123.png')), true, 'a local file');
  assert.equal(asksBeforeUpload(wp('abc123.png', { sharedId: 'k3f9x2ab', sharedWallpaper: 'abc123.png' })), false, 'shared with this picture');
  assert.equal(asksBeforeUpload(wp('abc123.png', { sharedId: 'k3f9x2ab', sharedWallpaper: 'old999.png' })), true, 'shared with another picture');
  assert.equal(asksBeforeUpload(wp('abc123.png', { sharedId: '', sharedWallpaper: 'abc123.png' })), true, 'not shared');
});
