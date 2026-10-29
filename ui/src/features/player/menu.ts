// V101-17 The menu every player shares, from the gear and from a right-click: one order and
// wording. Playback, then links and tools, then Twitch. Streams, videos (VODs) and clips leave out what
// they can't do.

import type { MenuEntry } from '../../components/menu.svelte';
import { formatFactor } from '../../lib/format.ts';

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
  /** V108-N01 VODs: clips the video at the current time; left out where it can't. */
  clipHere?: { run: () => void; busy?: boolean };
  /** V108-N02 Clips: saves the file as MP4; `text` while one saves (then picking stops it). */
  save?: { saving: string | null; start: () => void; stop: () => void };
  /** V108-N25 Streams: the moments found in chat, newest first. Left out without chat. */
  moments?: MomentRow[];
}

/** V108-N25 One moment in the list: when and what; each action only where it can be done. */
export interface MomentRow {
  time: string;
  text: string;
  goThere?: () => void;
  clip?: () => void;
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
      hint: speed.now === 1 ? 'Normal' : formatFactor(speed.now),
      items: SPEEDS.map((s) => ({ label: s === 1 ? 'Normal' : formatFactor(s), checked: s === speed.now, run: () => speed.set(s) })),
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
  );
  const clipHere = m.clipHere;
  if (clipHere) items.push({ label: 'Clip at current time', icon: 'scissors', disabled: clipHere.busy, run: clipHere.run });
  const save = m.save;
  if (save) {
    items.push(
      save.saving === null
        ? { label: 'Save as MP4', icon: 'download', run: save.start }
        : { label: 'Stop saving', icon: 'x', hint: save.saving, run: save.stop },
    );
  }
  if (m.moments) items.push(momentsEntry(m.moments));
  if (clipHere || save || m.moments) items.push('separator');
  items.push({ label: `Copy ${NOUN[m.kind]} link`, icon: 'link', run: () => m.copy(m.link) });
  const at = m.linkAt;
  if (at) items.push({ label: 'Copy link at current time', icon: 'copy', run: () => m.copy(at) });
  if (m.stats) items.push({ label: 'Video stats', icon: 'activity', checked: m.stats.on, run: m.stats.toggle });
  if (m.reload) items.push({ label: 'Reload player', icon: 'refresh', run: m.reload });
  items.push('separator', { label: 'Open on Twitch', icon: 'twitch', run: () => m.open(m.link) });
  return items;
}

/** The Moments list: one submenu per moment with what can be done there. */
function momentsEntry(rows: MomentRow[]): MenuEntry {
  if (!rows.length) return { label: 'Moments', icon: 'zap', hint: 'None yet', disabled: true };
  return {
    label: 'Moments',
    icon: 'zap',
    hint: String(rows.length),
    items: rows.map((r): MenuEntry => {
      const acts: MenuEntry[] = [];
      if (r.goThere) acts.push({ label: 'Go there', icon: 'history', run: r.goThere });
      if (r.clip) acts.push({ label: 'Clip', icon: 'scissors', note: 'Clips the seconds around it.', run: r.clip });
      return acts.length
        ? { label: r.text, hint: r.time, items: acts }
        : { label: r.text, hint: r.time, disabled: true, note: 'Too far back to go there.' };
    }),
  };
}
