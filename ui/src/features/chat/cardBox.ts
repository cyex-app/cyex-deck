// V110-C66 The box of a card under a message and the sizes of a hover preview's picture.
// The card owns its final box from the first frame (a skeleton the same size), so when the
// answer arrives, or its picture loads, no line moves.

import type { LinkThumbs } from '../../ipc/bindings/LinkThumbs';
import type { LinkPreview } from '../../ipc/bindings/LinkPreview';
import { previewPic } from './previewPic.ts';
import { factText } from './linkFacts.ts';

/** The card's picture box, px: always drawn at this size, loaded or not. */
export const CARD_PIC = { w: 96, h: 54 } as const;
/** Text lines in a card: title, one more, and the site line. */
export const CARD_LINES = 3;
/** Line height (unitless, from the chat) times the card's 0.85em, per line in em. */
const LINE_EM = 1.45;
/** The card's content height (CSS): the picture or the three text lines, whichever is taller.
 *  Written once here for both the skeleton and the card. */
export const CARD_HEIGHT = `max(${CARD_PIC.h}px, ${(CARD_LINES * LINE_EM).toFixed(2)}em)`;

/** Hover preview picture width in px by the "Preview size" setting (Chatterino's sizes). */
export const PREVIEW_PX: Record<LinkThumbs, number> = { off: 0, small: 100, medium: 200, large: 300 };

/** The card's text: the title, then the next line (subtitle, facts or the resolver's first line).
 *  V110-U70 A web page's second line is its subtitle, else its text (an X post's words); its facts
 *  go on the site line (`cardSite`). The box stays the same size. */
export function cardLines(info: LinkPreview | null): string[] {
  if (!info) return [];
  const rest = info.lines.filter((l) => l.length).map((l) => l.map((p) => p.text).join(''));
  const head = info.kind === 'web' ? [info.title, info.subtitle] : [info.title, info.subtitle, factsText(info)];
  return [...head, ...rest].filter((t): t is string => !!t).slice(0, CARD_LINES - 1);
}

function factsText(info: LinkPreview): string {
  return info.facts.map(factText).filter(Boolean).join(' · ');
}

/** V110-U70 The card's last line: a web page's host and its facts ("x.com · 325.006 likes · 2006/03/21"),
 *  else what kind of Twitch link it is (`kind`). */
export function cardSite(info: LinkPreview, host: string, kind: string): string {
  return info.kind === 'web' ? [host, factsText(info)].filter(Boolean).join(' · ') : kind;
}

/** V110-U70 The card's picture: the preview, else the poster's avatar (drawn round). */
export function cardPic(info: LinkPreview | null): { src: string; round: boolean } | null {
  const pic = previewPic(info?.thumbnail);
  if (pic) return { src: pic, round: false };
  const face = previewPic(info?.avatar);
  return face ? { src: face, round: true } : null;
}

/** V110-U70 A loaded picture's width in the card: its own shape at the card's height, never cropped,
 *  40 to 168 px (past that it is letterboxed, still whole). */
export function cardPicWidth(w: number, h: number): number {
  if (!(w > 0 && h > 0)) return CARD_PIC.w;
  return Math.round(Math.min(168, Math.max(40, (CARD_PIC.h * w) / h)));
}

/** wait: asking. ready: the card. plain: nothing to show, but it was on screen, so the box stays
 *  with just the site. gone: nothing to show and off screen, so the box goes (the list's scroll
 *  anchoring keeps what is read where it is). */
export type CardState = 'wait' | 'ready' | 'plain' | 'gone';

export function cardAfter(info: LinkPreview | null, onScreen: boolean): CardState {
  if (info && (cardLines(info).length || cardPic(info))) return 'ready';
  return onScreen ? 'plain' : 'gone';
}
