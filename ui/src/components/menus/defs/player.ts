// V101-17 V110-U66 The menu every player shares, from the gear and from a right-click.
// Streams, videos (VODs) and clips leave out what they can't do.

import { formatFactor } from '../../../lib/format.ts';
import type { MenuEntry } from '../../menuItems.ts';
import { group, item, type MenuDef, SEP } from '../types.ts';
import { noop, row, type Who } from './make.ts';

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
  /** V110-U66 Streams in a tab: theatre mode. */
  theatre?: { hint?: string; run: () => void };
}

/** V108-N25 One moment in the list: when and what; each action only where it can be done. */
export interface MomentRow {
  time: string;
  text: string;
  goThere?: () => void;
  clip?: () => void;
}

const NOUN = { live: 'stream', vod: 'video', clip: 'clip' } as const;

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

export const PLAYER_DEF: MenuDef<PlayerMenu> = {
  id: 'player',
  label: 'Players',
  where: 'Right-click a player or open its gear',
  items: [
    row('play', 'Play or pause', 'play', (m) => ({ label: m.playing ? 'Pause' : 'Play', icon: m.playing ? 'pause' : 'play', hint: m.keys.play, run: m.togglePlay }), {
      renamable: false,
    }),
    row('mute', 'Mute or unmute', 'volumeOff', (m) => ({ label: m.muted ? 'Unmute' : 'Mute', icon: m.muted ? 'volume' : 'volumeOff', hint: m.keys.mute, run: m.toggleMute }), {
      renamable: false,
    }),
    row('quality', 'Quality', 'sliders', (m) => {
      const now = m.quality.find((o) => o.on);
      return {
        label: 'Quality',
        icon: 'sliders',
        hint: now && (now.label === 'Auto' && now.hint ? `Auto · ${now.hint}` : now.label),
        items: m.quality.map((o) => ({ label: o.label, hint: o.hint, checked: o.on, run: o.pick })),
      };
    }),
    row('speed', 'Speed', 'clock', (m) => {
      const speed = m.speed;
      return speed
        ? {
            label: 'Speed',
            icon: 'clock',
            hint: speed.now === 1 ? 'Normal' : formatFactor(speed.now),
            items: SPEEDS.map((s) => ({ label: s === 1 ? 'Normal' : formatFactor(s), checked: s === speed.now, run: () => speed.set(s) })),
          }
        : null;
    }, { when: 'videos and clips' }),
    row('pip', 'Picture in picture', 'layers', (m) => (m.pip ? { label: 'Picture in picture', icon: 'layers', checked: m.pip.on, run: m.pip.toggle } : null)),
    row(
      'fullscreen',
      'Fullscreen',
      'expand',
      (m) => ({ label: m.fullscreen ? 'Exit fullscreen' : 'Fullscreen', icon: m.fullscreen ? 'shrink' : 'expand', hint: m.keys.fullscreen, run: m.toggleFullscreen }),
      { renamable: false },
    ),
    row('clipHere', 'Clip at current time', 'scissors', (m) => (m.clipHere ? { label: 'Clip at current time', icon: 'scissors', disabled: m.clipHere.busy, run: m.clipHere.run } : null), {
      when: 'videos',
    }),
    row(
      'save',
      'Save as MP4',
      'download',
      (m) =>
        m.save
          ? m.save.saving === null
            ? { label: 'Save as MP4', icon: 'download', run: m.save.start }
            : { label: 'Stop saving', icon: 'x', hint: m.save.saving, run: m.save.stop }
          : null,
      { renamable: false, when: 'clips' },
    ),
    row('moments', 'Moments', 'zap', (m) => (m.moments ? momentsEntry(m.moments) : null), { when: 'streams' }),
    row('copyLink', 'Copy link', 'link', (m) => ({ label: `Copy ${NOUN[m.kind]} link`, icon: 'link', run: () => m.copy(m.link) }), { renamable: false }),
    row('copyAt', 'Copy link at current time', 'copy', (m) => {
      const at = m.linkAt;
      return at ? { label: 'Copy link at current time', icon: 'copy', run: () => m.copy(at) } : null;
    }, { when: 'videos' }),
    row('stats', 'Video stats', 'activity', (m) => (m.stats ? { label: 'Video stats', icon: 'activity', checked: m.stats.on, run: m.stats.toggle } : null)),
    row('reload', 'Reload player', 'refresh', (m) => (m.reload ? { label: 'Reload player', icon: 'refresh', run: m.reload } : null)),
    row('open', 'Open on Twitch', 'twitch', (m) => ({ label: 'Open on Twitch', icon: 'twitch', run: () => m.open(m.link) })),
    // Palette only (3e).
    row('theatre', 'Theatre', 'expand', (m) => (m.theatre ? { label: 'Theatre', icon: 'expand', hint: m.theatre.hint, run: m.theatre.run } : null)),
  ],
  defaults: [
    item('play'),
    item('mute'),
    item('quality'),
    item('fullscreen'),
    item('clipHere'),
    SEP,
    item('speed'),
    item('pip'),
    item('moments'),
    SEP,
    group('share', 'Share', [item('copyLink'), item('copyAt'), item('open')], 'link'),
    SEP,
    group('tools', 'Tools', [item('stats'), item('reload'), item('save')], 'settings'),
  ],
};

/** Every row present, so a preview shows the whole menu; the `when` tags say which kind of player has which. */
export function playerSample(_who: Who): PlayerMenu {
  return {
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
    linkAt: 'https://www.twitch.tv/videos/1?t=5s',
    copy: noop,
    open: noop,
    clipHere: { run: noop },
    save: { saving: null, start: noop, stop: noop },
    moments: [{ time: '0:10', text: 'Chat spike', goThere: noop, clip: noop }],
    theatre: { run: noop },
  };
}
