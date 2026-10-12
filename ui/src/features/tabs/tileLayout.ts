// V103-05 A tile's panels as one panel layout (`lib/panels.ts`): its stream (or a clip
// beside a chat-only tile, 1.19), its chat and its channel's mod view (V102-26). Until the
// user arranges them by hand the layout comes from the chat's side (V102-23), the shares
// and where `fitModview` puts the mod view; arranged, it's kept with the tile. Pure, tested.

import { MIN, type Side } from '../../lib/fit.ts';
import { type Box, leaf, type Node, split, type Split } from '../../lib/panels.ts';

/** The order panels sit in the page; it never changes, so none of them restarts. */
export const TILE_PANELS = ['stream', 'chat', 'modview'];

export const TILE_MINS: Record<string, Box> = {
  stream: { w: MIN.streamW, h: MIN.streamH },
  chat: { w: MIN.chatW, h: MIN.chatH },
  modview: { w: MIN.modviewW, h: MIN.modviewH },
};

export const tileMin = (id: string): Box => TILE_MINS[id] ?? { w: 0, h: 0 };

export interface TileParts {
  stream: boolean;
  chat: boolean;
  modview: boolean;
  /** The chat's side and share (V102-23, V102-24). */
  side: Side;
  ratio: number;
  /** The mod view beside the stream (left of it) or under it, and its share (V102-26). */
  modRow: boolean;
  modShare: number;
}

/** The layout a tile has without one made by hand: the mod view splits the stream's area,
 * the chat sits on its side of both. */
export function tileTree(p: TileParts): Node | null {
  let core: Node | null = p.stream ? leaf('stream') : null;
  if (p.modview) {
    const s = p.modShare;
    core = core
      ? split(p.modRow ? 'row' : 'column', p.modRow ? [leaf('modview'), core] : [core, leaf('modview')], p.modRow ? [s, 1 - s] : [1 - s, s])
      : leaf('modview');
  }
  if (!p.chat) return core;
  if (!core) return leaf('chat');
  const row = p.side === 'left' || p.side === 'right';
  const first = p.side === 'left' || p.side === 'top';
  return split(row ? 'row' : 'column', first ? [leaf('chat'), core] : [core, leaf('chat')], first ? [p.ratio, 1 - p.ratio] : [1 - p.ratio, p.ratio]);
}

/** The chat's side when `n` is just a stream and a chat (V102-23 keeps that as the chat's
 * side, not as a hand-made layout); else `null`. */
export function chatSideOf(n: Node | null): Side | null {
  if (!n || !('dir' in n) || n.parts.length !== 2) return null;
  const ids = n.parts.map((p) => ('panel' in p ? p.panel : null));
  if (!ids.includes('chat') || !ids.includes('stream')) return null;
  const chatFirst = ids[0] === 'chat';
  return n.dir === 'row' ? (chatFirst ? 'left' : 'right') : chatFirst ? 'top' : 'bottom';
}

/** Which of the tile's own shares a split is: the chat's beside the rest, or the mod view's
 * beside the stream; `null` for any other. `index` is the part the share belongs to. */
export function shareOf(s: Split): { kind: 'chat' | 'modview'; index: number } | null {
  const ids = s.parts.map((p) => ('panel' in p ? p.panel : null));
  if (s.parts.length !== 2) return null;
  const chat = ids.indexOf('chat');
  if (chat >= 0) return { kind: 'chat', index: chat };
  const mod = ids.indexOf('modview');
  if (mod >= 0 && ids.includes('stream')) return { kind: 'modview', index: mod };
  return null;
}

/** V102-24 A split of the stream and a chat turned round to fit: the stream at its full
 * width's 16:9 height above the chat, or a 28 % chat beside a stream too short for one
 * under it. Other splits keep their shares. */
export function turnedShares(s: Split, box: Box): number[] | null {
  const ids = s.parts.map((p) => ('panel' in p ? p.panel : null));
  const chat = ids.indexOf('chat');
  if (s.parts.length !== 2 || chat < 0 || box.w <= 0 || box.h <= 0) return null;
  // Turning to a column (it was a row) puts the chat under the stream.
  const share = s.dir === 'row' ? Math.max(MIN.chatH, box.h - (box.w * 9) / 16) / box.h : 0.28;
  return chat === 0 ? [share, 1 - share] : [1 - share, share];
}
