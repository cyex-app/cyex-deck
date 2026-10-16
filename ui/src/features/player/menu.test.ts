import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { MenuEntry } from '../../components/menu.svelte';
import { type PlayerMenu, playerMenu } from './menu.ts';

const noop = () => {};

function base(kind: PlayerMenu['kind']): PlayerMenu {
  return {
    kind,
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
    link: 'https://www.twitch.tv/xqc',
    copy: noop,
    open: noop,
  };
}

const labels = (list: MenuEntry[]) => list.map((e) => (e === 'separator' ? '—' : e.label));

test('a stream: playback, then links and tools, then twitch.tv', () => {
  const m = { ...base('live'), pip: { on: false, toggle: noop }, stats: { on: true, toggle: noop }, reload: noop };
  const list = playerMenu(m);
  assert.deepEqual(labels(list), [
    'Pause',
    'Mute',
    'Quality',
    'Picture in picture',
    'Fullscreen',
    '—',
    'Copy stream link',
    'Video stats',
    'Reload player',
    '—',
    'Open on Twitch',
  ]);
  const quality = list[2];
  assert.ok(quality !== 'separator');
  assert.equal(quality.hint, 'Auto · 720p60');
  assert.deepEqual(quality.items?.map((e) => e !== 'separator' && e.checked), [true, false]);
  const stats = list[7];
  assert.ok(stats !== 'separator' && stats.checked);
});

test('a video adds speed and the link at the current time', () => {
  const m = {
    ...base('vod'),
    playing: false,
    muted: true,
    fullscreen: true,
    speed: { now: 1.5, set: noop },
    pip: { on: false, toggle: noop },
    stats: { on: false, toggle: noop },
    reload: noop,
    link: 'https://www.twitch.tv/videos/1',
    linkAt: 'https://www.twitch.tv/videos/1?t=1m5s',
  };
  assert.deepEqual(labels(playerMenu(m)), [
    'Play',
    'Unmute',
    'Quality',
    'Speed',
    'Picture in picture',
    'Exit fullscreen',
    '—',
    'Copy video link',
    'Copy link at current time',
    'Video stats',
    'Reload player',
    '—',
    'Open on Twitch',
  ]);
});

test('a clip keeps it short', () => {
  let opened = '';
  const list = playerMenu({ ...base('clip'), link: 'https://clips.twitch.tv/A', open: (u) => (opened = u) });
  assert.deepEqual(labels(list), ['Pause', 'Mute', 'Quality', 'Fullscreen', '—', 'Copy clip link', '—', 'Open on Twitch']);
  const last = list.at(-1);
  assert.ok(last && last !== 'separator');
  last.run?.();
  assert.equal(opened, 'https://clips.twitch.tv/A');
});
