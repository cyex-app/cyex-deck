import assert from 'node:assert/strict';
import { test } from 'node:test';

import { playerMenu, type PlayerMenu } from '../features/player/menu.ts';
import { type MenuEntry, menuProblems, tidy } from './menuItems.ts';

const item = (label: string, more: object = {}) => ({ label, icon: 'copy' as const, ...more });

test('V101-30 parts that left nothing leave no stray lines', () => {
  const list: MenuEntry[] = ['separator', item('A'), 'separator', 'separator', item('B'), 'separator'];
  assert.deepEqual(tidy(list), [item('A'), 'separator', item('B')]);
  assert.deepEqual(tidy(['separator']), []);
});

test('V101-30 the rules catch what menus used to do', () => {
  assert.deepEqual(menuProblems([item('Copy name'), item('Hide')]), []);
  assert.deepEqual(menuProblems([item('Open'), { label: 'Open in the background' }]), ['icons on some items only']);
  assert.deepEqual(menuProblems([item('Ban', { danger: true }), item('Warn')]), ['"Ban" isn\'t last in its group']);
  assert.deepEqual(menuProblems([item('Star', { hint: 'Keeps it on top of home' })]), [
    'hint "Keeps it on top of home" explains; use note',
  ]);
  assert.deepEqual(menuProblems([item('copy link.')]), ['label "copy link."']);
});

test('V101-30 the player menu keeps the rules', () => {
  const noop = () => {};
  const m: PlayerMenu = {
    kind: 'vod',
    playing: true,
    muted: false,
    fullscreen: false,
    keys: { play: 'Space', mute: 'M', fullscreen: 'F' },
    quality: [
      { label: 'Auto', hint: '720p60', on: true, pick: noop },
      { label: '1080p60', hint: 'Source', on: false, pick: noop },
    ],
    togglePlay: noop,
    toggleMute: noop,
    toggleFullscreen: noop,
    pip: { on: false, toggle: noop },
    speed: { now: 1, set: noop },
    stats: { on: false, toggle: noop },
    reload: noop,
    link: 'https://www.twitch.tv/videos/1',
    linkAt: 'https://www.twitch.tv/videos/1?t=1m',
    copy: noop,
    open: noop,
  };
  // Pick lists (quality, speed) have checks, not icons.
  const top = playerMenu(m).map((e) => (e === 'separator' || !e.items ? e : { ...e, items: undefined }));
  assert.deepEqual(menuProblems(top), []);
});
