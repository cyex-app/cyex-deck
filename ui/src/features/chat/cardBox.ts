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

/** The card's text: the title, then the next line (subtitle, facts or the resolver's first line). */
export function cardLines(info: LinkPreview | null): string[] {
  if (!info) return [];
  const facts = info.facts.map(factText).filter(Boolean).join(' · ');
  const rest = info.lines.filter((l) => l.length).map((l) => l.map((p) => p.text).join(''));
  return [info.title, info.subtitle, facts, ...rest].filter((t): t is string => !!t).slice(0, CARD_LINES - 1);
}

/** wait: asking. ready: the card. plain: nothing to show, but it was on screen, so the box stays
 *  with just the site. gone: nothing to show and off screen, so the box goes (the list's scroll
 *  anchoring keeps what is read where it is). */
export type CardState = 'wait' | 'ready' | 'plain' | 'gone';

export function cardAfter(info: LinkPreview | null, onScreen: boolean): CardState {
  if (info && (cardLines(info).length || previewPic(info.thumbnail))) return 'ready';
  return onScreen ? 'plain' : 'gone';
}
