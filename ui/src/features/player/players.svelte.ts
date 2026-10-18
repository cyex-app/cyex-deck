// Players on screen and what each tile shows next to its chat. Player hotkeys (S2), media
// keys and Windows' media controls (V101-29) go to the player used or opened last. A tile's state is shared by its stream side and its chat side:
// pages (1.21), a clip over the stream (1.20), a clip or VOD beside a chat-only tile
// (1.19, P7), the VOD the chat replays (P8).

import { getCurrentWindow } from '@tauri-apps/api/window';

import type { LinkTarget } from '../../ipc/bindings/LinkTarget';
import type { TileShow } from '../../ipc/bindings/TileShow';
import { settings } from '../../lib/settings.svelte';
import { currentWindow } from '../../lib/window.svelte';
import { liveKeyAction, ownsKey, type SeekResult, seekText, stepChoice, Steps } from './liveKeys';

/** V101-29 What Windows' media controls show for a player. */
export interface MediaInfo {
  title: string;
  artist: string;
  artwork?: string | null;
}

export interface PlayerControls {
  togglePlay(): void;
  toggleMute(): void;
  toggleFullscreen(): void;
  /** V101-29 Media keys. */
  play?(): void;
  pause?(): void;
  /** Seconds ahead, negative to go back. VODs and clips return nothing; a live stream says
   *  where it landed, or null when it can't seek (V1061-04). */
  seekBy?(seconds: number): unknown;
  /** V1061-02 Live streams: back to the live point, playing. */
  toLive?(): void;
  /** Q25 Â±1 = Â±5 %. */
  volumeStep?(direction: number): void;
  /** Whether the player is on screen (not hidden, not the corner player). */
  visible?(): boolean;
  /** Shows a short readout over the picture. */
  say?(text: string): void;
  /** Read in an effect: follows the player's state. */
  media?(): MediaInfo | null;
}

class Players {
  private list = $state.raw<PlayerControls[]>([]);
  private last = $state.raw<PlayerControls | null>(null);

  /** A player that opens takes over, e.g. a clip over the stream. Returns the function
   *  that takes it off the list. */
  register(p: PlayerControls): () => void {
    this.list = [...this.list, p];
    this.last = p;
    return () => {
      this.list = this.list.filter((x) => x !== p);
      if (this.last === p) this.last = null;
    };
  }

  use(p: PlayerControls): void {
    this.last = p;
  }

  get active(): PlayerControls | null {
    return this.last ?? this.list[0] ?? null;
  }

  /** Q27 a The player the arrow keys act on: the one used last if it is on screen, else the first that is. */
  get shown(): PlayerControls | null {
    const l = this.last;
    if (l?.visible?.()) return l;
    return this.list.find((p) => p.visible?.()) ?? null;
  }
}

export const players = new Players();

/** S2 Player hotkeys, for `useHotkeys`. A key on a focused control stays with it. */
export const playerHotkeys = {
  'player.playPause': () => withPlayer((p) => p.togglePlay()),
  'player.mute': () => withPlayer((p) => p.toggleMute()),
  'player.fullscreen': () => withPlayer((p) => p.toggleFullscreen()),
  'player.back': (e: KeyboardEvent) => keySeek(-1, e),
  'player.forward': (e: KeyboardEvent) => keySeek(1, e),
  'player.back2': (e: KeyboardEvent) => keySeek(-1, e),
  'player.forward2': (e: KeyboardEvent) => keySeek(1, e),
  'player.live': (e: KeyboardEvent) => goLive(e),
  'player.volumeUp': (e: KeyboardEvent) => keyVolume(1, e),
  'player.volumeDown': (e: KeyboardEvent) => keyVolume(-1, e),
  'player.playPause2': () => {
    const p = players.shown;
    if (!p) return false;
    p.togglePlay();
  },
};

const steps = new Steps();

/** V1061-04 Left/Right/J/L. Returns false (key stays with the page) when nothing can seek. */
function keySeek(sign: number, e: KeyboardEvent): void | false {
  const p = players.shown;
  if (!p?.seekBy || ownsKey(document.activeElement, e)) return false;
  const step = stepChoice(settings.v.player.seekStep);
  const raw = p.seekBy(sign * step);
  const r = raw === null ? null : raw && typeof raw === 'object' && 'behind' in raw ? (raw as SeekResult) : undefined;
  const t = seekText(r, steps.add(sign * step, performance.now()));
  if (t === null) return false;
  p.say?.(t);
}

/** Q25 Up/Down: volume Â±5 %. */
function keyVolume(d: number, e: KeyboardEvent): void | false {
  const p = players.shown;
  if (!p?.volumeStep || ownsKey(document.activeElement, e)) return false;
  p.volumeStep(d);
}

/** V1061-05 Shift+Right. */
function goLive(e: KeyboardEvent): void | false {
  const p = players.shown;
  const a = liveKeyAction({ repeat: e.repeat, owns: ownsKey(document.activeElement, e), hasLive: !!p?.toLive });
  if (a === 'pass') return false;
  if (a === 'skip') return;
  p!.toLive!();
  p!.say?.('Live');
}

/** Focused controls that use Space, M or F themselves. */
const CONTROLS =
  'button, a, select, [role="slider"], [role="radio"], [role="tab"], [role="option"], [role="listbox"], [role^="menuitem"], [role="checkbox"], [role="switch"]';

function withPlayer(f: (p: PlayerControls) => void): void | false {
  const p = players.active;
  const focus = document.activeElement;
  if (!p || (focus instanceof HTMLElement && focus.closest(CONTROLS))) {
    return false;
  }
  f(p);
}

/** What the stream side of a tile shows. */
export type Page =
  | { kind: 'stream' }
  | { kind: 'about' }
  | { kind: 'clips' }
  | { kind: 'videos' }
  | { kind: 'vod'; id: string; at: number }
  | { kind: 'clip'; slug: string };

export class TileState {
  page = $state<Page>({ kind: 'stream' });
  /** 1.20 A clip over the stream, which is muted meanwhile. */
  overlay = $state<string | null>(null);
  /** 1.19, P7 A clip or VOD beside a chat-only tile. */
  side = $state<LinkTarget | null>(null);
  /** P8 The VOD playing in this tile and where it is. */
  vod = $state<{ id: string; time: number } | null>(null);
  /** P8 The chat shows the VOD's chat while one plays. */
  replay = $state(true);
  /** Stories open over the stream. */
  stories = $state(false);
}

const states = new Map<string, TileState>();

export function tileState(id: string): TileState {
  let s = states.get(id);
  if (!s) {
    s = new TileState();
    states.set(id, s);
  }
  return s;
}

/** A clip or VOD from chat, in the tile it was clicked in: over the stream (1.20) or among
 *  the channel's videos (P7) when the tile shows the stream, beside the chat otherwise (1.19). */
export function openTarget(tile: { id: string; show: TileShow }, target: LinkTarget): void {
  const s = tileState(tile.id);
  if (!tile.show.player) s.side = target;
  else if (target.kind === 'clip') s.overlay = target.slug;
  else s.page = { kind: 'vod', id: target.id, at: target.at };
}

/** 1.58 Fullscreen: the element fills the window and the window fills the screen. A
 *  player inside a `[data-fullscreen]` surface takes the surface along, so a clip over the
 *  stream still shows (V101-26). */
export async function toggleFullscreen(el: HTMLElement): Promise<void> {
  if (document.fullscreenElement) await document.exitFullscreen();
  else await el.requestFullscreen();
}

document.addEventListener('fullscreenchange', () => {
  // V104-03 A pop-out the frame made fullscreen stays so when the player leaves its own.
  void getCurrentWindow().setFullscreen(Boolean(document.fullscreenElement) || currentWindow.fullscreen);
});
