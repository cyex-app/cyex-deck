// Players on screen and what each tile shows next to its chat. Player hotkeys (S2) go to
// the player used last. A tile's state is shared by its stream side and its chat side:
// pages (1.21), a clip over the stream (1.20), a clip or VOD beside a chat-only tile
// (1.19, P7), the VOD the chat replays (P8).

import { getCurrentWindow } from '@tauri-apps/api/window';

import type { LinkTarget } from '../../ipc/bindings/LinkTarget';
import type { TileShow } from '../../ipc/bindings/TileShow';

export interface PlayerControls {
  togglePlay(): void;
  toggleMute(): void;
  toggleFullscreen(): void;
}

class Players {
  private list: PlayerControls[] = [];
  private last: PlayerControls | null = null;

  /** Returns the function that takes the player off the list. */
  register(p: PlayerControls): () => void {
    this.list.push(p);
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
}

export const players = new Players();

/** S2 Player hotkeys, for `useHotkeys`. A key on a focused control stays with it. */
export const playerHotkeys = {
  'player.playPause': () => withPlayer((p) => p.togglePlay()),
  'player.mute': () => withPlayer((p) => p.toggleMute()),
  'player.fullscreen': () => withPlayer((p) => p.toggleFullscreen()),
};

function withPlayer(f: (p: PlayerControls) => void): void | false {
  const p = players.active;
  const focus = document.activeElement;
  if (!p || (focus instanceof HTMLElement && focus.closest('button, a, select, [role="slider"], [role="radio"]'))) {
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

/** 1.58 Fullscreen: the element fills the window and the window fills the screen. */
export async function toggleFullscreen(el: HTMLElement): Promise<void> {
  if (document.fullscreenElement) await document.exitFullscreen();
  else await el.requestFullscreen();
}

document.addEventListener('fullscreenchange', () => {
  void getCurrentWindow().setFullscreen(Boolean(document.fullscreenElement));
});
