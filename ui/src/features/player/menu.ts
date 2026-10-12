// V101-17 The right-click menu every player shares, in one order and wording: playback,
// then links and tools, then twitch.tv. Streams, videos (VODs) and clips leave out what
// they can't do.

import type { MenuEntry } from '../../components/menu.svelte';

export const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 2];

export interface PlayerMenu {
  kind: 'live' | 'vod' | 'clip';
  playing: boolean;
  muted: boolean;
  fullscreen: boolean;
  /** Hotkeys as shown, '' when unbound. */
  keys: { play: string; mute: string; fullscreen: string };
  /** The quality list; `on` marks the one picked, `hint` what "Auto" plays now. */
  quality: { label: string; hint?: string; on: boolean; pick: () => void }[];
  togglePlay: () => void;
  toggleMute: () => void;
  toggleFullscreen: () => void;
  /** Left out where the player can't. */
  pip?: { on: boolean; toggle: () => void };
  speed?: { now: number; set: (s: number) => void };
  stats?: { on: boolean; toggle: () => void };
  reload?: () => void;
  /** The stream, video or clip on twitch.tv. */
  link: string;
  /** VODs: the link at the current time. */
  linkAt?: string;
  copy: (text: string) => void;
  open: (url: string) => void;
}

const NOUN = { live: 'stream', vod: 'video', clip: 'clip' } as const;

export function playerMenu(m: PlayerMenu): MenuEntry[] {
  const now = m.quality.find((o) => o.on);
  const items: MenuEntry[] = [
    { label: m.playing ? 'Pause' : 'Play', icon: m.playing ? 'pause' : 'play', hint: m.keys.play, run: m.togglePlay },
    { label: m.muted ? 'Unmute' : 'Mute', icon: m.muted ? 'volume' : 'volumeOff', hint: m.keys.mute, run: m.toggleMute },
    {
      label: 'Quality',
      icon: 'sliders',
      hint: now && (now.label === 'Auto' && now.hint ? `Auto · ${now.hint}` : now.label),
      items: m.quality.map((o) => ({ label: o.label, hint: o.hint, checked: o.on, run: o.pick })),
    },
  ];
  const speed = m.speed;
  if (speed) {
    items.push({
      label: 'Speed',
      icon: 'clock',
      hint: `${speed.now}×`,
      items: SPEEDS.map((s) => ({ label: s === 1 ? 'Normal' : `${s}×`, checked: s === speed.now, run: () => speed.set(s) })),
    });
  }
  if (m.pip) items.push({ label: 'Picture in picture', icon: 'layers', checked: m.pip.on, run: m.pip.toggle });
  items.push(
    {
      label: m.fullscreen ? 'Exit fullscreen' : 'Fullscreen',
      icon: m.fullscreen ? 'shrink' : 'expand',
      hint: m.keys.fullscreen,
      run: m.toggleFullscreen,
    },
    'separator',
    { label: `Copy ${NOUN[m.kind]} link`, icon: 'link', run: () => m.copy(m.link) },
  );
  const at = m.linkAt;
  if (at) items.push({ label: 'Copy link at current time', icon: 'copy', run: () => m.copy(at) });
  if (m.stats) items.push({ label: 'Video stats', icon: 'activity', checked: m.stats.on, run: m.stats.toggle });
  if (m.reload) items.push({ label: 'Reload player', icon: 'refresh', run: m.reload });
  items.push('separator', { label: 'Open on twitch.tv', icon: 'popout', run: () => m.open(m.link) });
  return items;
}
