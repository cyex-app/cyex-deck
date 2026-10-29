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
  const speed = playerMenu(m)[3];
  assert.ok(speed !== 'separator');
  assert.equal(speed.hint, '1,5×');
  assert.deepEqual(speed.items?.map((e) => e !== 'separator' && e.label), ['0,5×', '0,75×', 'Normal', '1,25×', '1,5×', '2×']);
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

test('a video clips at the current time, before the links', () => {
  let ran = 0;
  const list = playerMenu({ ...base('vod'), linkAt: 'https://www.twitch.tv/videos/1?t=5s', clipHere: { run: () => ran++ } });
  assert.deepEqual(labels(list), [
    'Pause', 'Mute', 'Quality', 'Fullscreen', '—', 'Clip at current time', '—', 'Copy video link', 'Copy link at current time', '—', 'Open on Twitch',
  ]);
  const clip = list[5];
  assert.ok(clip !== 'separator');
  clip.run?.();
  assert.equal(ran, 1);
  const busy = playerMenu({ ...base('vod'), clipHere: { run: noop, busy: true } })[5];
  assert.ok(busy !== 'separator' && busy.disabled);
});

test('a clip saves as MP4 and a running save can be stopped', () => {
  const idle = playerMenu({ ...base('clip'), save: { saving: null, start: noop, stop: noop } });
  assert.deepEqual(labels(idle), ['Pause', 'Mute', 'Quality', 'Fullscreen', '—', 'Save as MP4', '—', 'Copy clip link', '—', 'Open on Twitch']);
  const busy = playerMenu({ ...base('clip'), save: { saving: 'Saving 42 %', start: noop, stop: noop } })[5];
  assert.ok(busy !== 'separator');
  assert.equal(busy.label, 'Stop saving');
  assert.equal(busy.hint, 'Saving 42 %');
});

test('moments: Go there and Clip only where they can be done', () => {
  const calls: string[] = [];
  const rows = [
    { time: '1:02:03', text: 'Chat spike · KEKW', goThere: () => calls.push('go'), clip: () => calls.push('clip') },
    { time: '0:10', text: 'Raid', clip: () => calls.push('clip2') },
    { time: '0:05', text: 'Hype train' },
  ];
  const list = playerMenu({ ...base('live'), moments: rows });
  const entry = list.find((e) => e !== 'separator' && e.label === 'Moments');
  assert.ok(entry && entry !== 'separator');
  assert.equal(entry.hint, '3');
  const [first, second, third] = entry.items ?? [];
  assert.ok(first && first !== 'separator' && second && second !== 'separator' && third && third !== 'separator');
  assert.deepEqual(first.items?.map((e) => e !== 'separator' && e.label), ['Go there', 'Clip']);
  assert.deepEqual(second.items?.map((e) => e !== 'separator' && e.label), ['Clip']);
  assert.equal(third.disabled, true);
  assert.equal(third.items, undefined);
  const go = first.items?.[0];
  assert.ok(go && go !== 'separator');
  go.run?.();
  assert.deepEqual(calls, ['go']);
});

test('no moments yet shows a quiet entry', () => {
  const entry = playerMenu({ ...base('live'), moments: [] }).find((e) => e !== 'separator' && e.label === 'Moments');
  assert.ok(entry && entry !== 'separator');
  assert.equal(entry.disabled, true);
  assert.equal(entry.hint, 'None yet');
});
