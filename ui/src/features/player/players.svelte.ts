// Players on screen and what each tile shows next to its chat. Player hotkeys (S2), media
// keys and Windows' media controls (V101-29) go to the player used or opened last. A tile's state is shared by its stream side and its chat side:
// pages (1.21), a clip over the stream (1.20), a clip or VOD beside a chat-only tile
// (1.19, P7), the VOD the chat replays (P8).

import { getCurrentWindow } from '@tauri-apps/api/window';

import type { LinkTarget } from '../../ipc/bindings/LinkTarget';
import type { TileShow } from '../../ipc/bindings/TileShow';
import { CappedMap, useCapped } from '../../lib/capped';
import { keyTaken } from '../../lib/escape';
import { settings } from '../../lib/settings.svelte';
import { pickShown } from './pickPlayer';
import { barKeepsKey, liveKeyAction, ownsKey, type SeekResult, seekText, stepChoice, Steps } from './liveKeys';

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
  /** Q25 ±1 = ±5 %. */
  volumeStep?(direction: number): void;
  /** Whether the player is on screen (not hidden, not the corner player). */
  visible?(): boolean;
  /** Shows a short readout over the picture. */
  say?(text: string): void;
  /** Read in an effect: follows the player's state. */
  media?(): MediaInfo | null;
}

class Players {
  // V108-P3 Plain fields, not state: a player opening over another (a clip over the stream) writes
  // the list while the old one's teardown reads it, and a read-modify-write on state lost the
  // new player, so the keys reached nothing until a click. `version` only tells effects to look again.
  private list: PlayerControls[] = [];
  private last: PlayerControls | null = null;
  private version = $state.raw({});

  /** A player that opens takes over, e.g. a clip over the stream. Returns the function
   *  that takes it off the list. */
  register(p: PlayerControls): () => void {
    this.list = [...this.list, p];
    this.last = p;
    this.version = {};
    return () => {
      this.list = this.list.filter((x) => x !== p);
      if (this.last === p) this.last = null;
      this.version = {};
    };
  }

  use(p: PlayerControls): void {
    this.last = p;
    this.version = {};
  }

  get active(): PlayerControls | null {
    void this.version;
    return this.last ?? this.list[0] ?? null;
  }

  /** Q27 a The player the arrow keys act on: the one used last if it is on screen, else the first that is. */
  get shown(): PlayerControls | null {
    void this.version;
    return pickShown(this.last, this.list);
  }
}

export const players = new Players();

/** S2 Player hotkeys, for `useHotkeys`. A key on a focused control stays with it. */
export const playerHotkeys = {
  'player.playPause': (e: KeyboardEvent) => withPlayer((p) => p.togglePlay(), e),
  'player.mute': (e: KeyboardEvent) => withPlayer((p) => p.toggleMute(), e),
  'player.fullscreen': (e: KeyboardEvent) => withPlayer((p) => p.toggleFullscreen(), e),
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

/** Q25 Up/Down: volume ±5 %. */
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

/** Focused controls that use Space, M or F themselves. Sliders use only arrows,
 * so Space and M still play and mute after the volume or seek bar was clicked. */
const CONTROLS =
  'button, a, select, [role="radio"], [role="tab"], [role="option"], [role="listbox"], [role^="menuitem"], [role="checkbox"], [role="switch"]';

// V108-F43 A button in the player bar presses with Space and Enter only; M, F and the rest reach the player.
function withPlayer(f: (p: PlayerControls) => void, e: KeyboardEvent): void | false {
  // V110-F83 Space, M and F act on a player you can see; only when none is on screen do they reach
  // the one playing in a background tab (as the media keys do, see `active`).
  const p = players.shown ?? players.active;
  const focus = document.activeElement;
  if (!p || (focus instanceof HTMLElement && focus.closest(CONTROLS) && (!focus.closest('[data-bar]') || barKeepsKey(focus, e)))) {
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

/** L1 Capped: closed tiles never come back, and a use keeps a tile among the newest. */
const states = new CappedMap<string, TileState>(200);

export function tileState(id: string): TileState {
  let s = useCapped(states, id);
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
  // V110-F83 A refused fullscreen isn't worth a message.
  if (document.fullscreenElement) await document.exitFullscreen().catch(() => {});
  else await el.requestFullscreen().catch(() => {});
}

/** rd-5 Whether a player is fullscreen, for the buttons' labels. */
export const playerFullscreen = $state({ on: false });

document.addEventListener('fullscreenchange', () => {
  playerFullscreen.on = Boolean(document.fullscreenElement);
  // V107-R46 Pop-outs maximize instead of going fullscreen, so only the player decides here.
  getCurrentWindow().setFullscreen(Boolean(document.fullscreenElement)).catch(() => {});
});

// rd-5 Escape leaves a player's fullscreen when WebView2 doesn't. Layers (menus, popovers)
// take their Escape first in the capture phase and use it up.
window.addEventListener('keydown', (e) => {
  if (e.key !== 'Escape' || e.isComposing || !document.fullscreenElement || keyTaken(e)) return;
  e.preventDefault();
  void document.exitFullscreen().catch(() => {});
});
