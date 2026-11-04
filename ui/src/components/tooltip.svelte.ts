// Tooltips (3.6). One shared tip for the whole window. The label always shows; the
// explanation underneath is the "new user" part and follows the tooltips setting (2.7).

import { untrack } from 'svelte';

import type { LinkPreview } from '../ipc/bindings/LinkPreview';
import type { Rect, Side } from '../lib/place';
import { settings } from '../lib/settings.svelte';
import { focusTipAllowed, tipAllowed } from './tooltipGate';
import { tipSlot } from './tipSlot';

export interface TipOptions {
  label: string;
  hint?: string;
  /** E6 A bigger picture above the label, e.g. an emote's. */
  image?: string;
  /** A 7TV paint's class: the label is drawn in that paint. */
  paint?: string;
  /** Shows only when this says so at that moment, e.g. while a label is cut off. */
  when?: () => boolean;
  /** ct-9 Where it goes when there's room; below by default. */
  side?: Side;
  /** V108-C25 A link's card under the label, e.g. loading until it's known. */
  card?: TipCard;
}

/** 1.52, V108-C25 A link's card: on its way, nothing to show, or the card. */
export type TipCard =
  | { state: 'loading' }
  | { state: 'none' }
  /** `thumbWidth` 0 hides the picture. */
  | { state: 'ready'; card: LinkPreview; thumbWidth: number; blur: boolean };

export const tip = $state({
  label: '',
  hint: '',
  image: '',
  paint: '',
  card: null as TipCard | null,
  /** The element it shows for, as it was when the tip opened. */
  anchor: null as Rect | null,
  side: 'below' as Side,
  visible: false,
});

const SHOW_DELAY_MS = 150;
/** Moving between buttons right after a tip closed shows the next one instantly. */
const WARM_MS = 400;

let lastKey: string | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;
let lastHidden = 0;
/** The element the tip shows (or is about to show) for: only it may retitle it. */
let owner: HTMLElement | null = null;
/** V108-C25 Cards and titles that arrive before the tip shows wait here for it. */
const cards = tipSlot<HTMLElement, TipCard>();
const titles = tipSlot<HTMLElement, string>();

/** For lists that delegate: one listener on the list instead of one per item. */
export function showTip(node: HTMLElement, opts: TipOptions): void {
  show(node, opts);
}

export function hideTip(): void {
  hide();
}

/** Changes the text of the tip `node` shows (or is about to), e.g. once a link resolves. */
export function retitleTip(node: HTMLElement, label: string): void {
  if (titles.put(node, label) && tip.visible) tip.label = label;
}

/** 1.52 Sets the card of the tip `node` shows (or is about to), once it's known. */
export function previewTip(node: HTMLElement, card: TipCard): void {
  if (cards.put(node, card) && tip.visible) tip.card = card;
}

function hintFor(opts: TipOptions): string {
  return opts.image ? (opts.hint ?? '') : settings.v.general?.tooltips ? (opts.hint ?? '') : '';
}

function show(node: HTMLElement, opts: TipOptions): void {
  clearTimeout(timer);
  // Nothing to say (e.g. a menu item without a note), or a drag is on (rb-1).
  if (!tipAllowed(opts, dragging())) return;
  owner = node;
  cards.claim(node, opts.card);
  titles.claim(node);
  const delay = performance.now() - lastHidden < WARM_MS ? 0 : SHOW_DELAY_MS;
  timer = setTimeout(() => {
    // rb-1 A drag that started during the delay.
    if (dragging() || owner !== node) return;
    tip.label = titles.read(node) ?? opts.label;
    tip.hint = hintFor(opts);
    tip.image = opts.image ?? '';
    tip.paint = opts.paint ?? '';
    tip.card = cards.read(node) ?? null;
    tip.anchor = node.getBoundingClientRect();
    tip.side = opts.side ?? 'below';
    tip.visible = true;
  }, delay);
}

/** rb-1 The shared drag (features/tabs/drag) marks the body while a drag is on. */
const dragging = () => typeof document !== 'undefined' && document.body.classList.contains('dragging');

function hide(): void {
  clearTimeout(timer);
  // Untracked: a layer that moves focus in its effect blurs a tipped trigger, and must not
  // re-run (and steal focus back) each time a tip shows inside it.
  if (untrack(() => tip.visible)) lastHidden = performance.now();
  owner = null;
  cards.release();
  titles.release();
  // C5 A trigger torn down inside an effect or derived (a menu closing) fires focusout/mouseleave
  // while Svelte forbids state writes; the write waits a microtask, which is after the teardown.
  if (!untrack(() => tip.visible)) return;
  queueMicrotask(() => {
    if (!owner) tip.visible = false;
  });
}

// A scroll that moves the tip's element hides it, a resize always does, and so does Escape. Added at load, before
// the Escape layer stack (lib/escape.ts) listens, which uses the key up when it closes a layer.
if (typeof window !== 'undefined') {
  const moved = () => {
    if (!tip.visible || !owner || !tip.anchor) return;
    const r = owner.getBoundingClientRect();
    if (Math.abs(r.top - tip.anchor.top) > 1 || Math.abs(r.left - tip.anchor.left) > 1) hide();
  };
  addEventListener('scroll', moved, { capture: true, passive: true });
  // A resize reflows the page under it, so it hides too.
  addEventListener('resize', hide);
  // Doesn't use the key up: Escape still closes what's under the tip.
  addEventListener('keydown', (e) => e.key === 'Escape' && tip.visible && hide(), true);
  // V108-F59i The last key (null after a pointer), so focus handed back after Escape or Enter shows no tip.
  addEventListener('keydown', (e) => (lastKey = e.key), true);
  addEventListener('pointerdown', () => (lastKey = null), true);
}

/** `use:tooltip={{ label, hint }}` or `use:tooltip={'Label'}`. Shows on hover and on
 *  keyboard focus; `update` refreshes the visible tip. */
export function tooltip(node: HTMLElement, initial: TipOptions | string) {
  let opts = typeof initial === 'string' ? { label: initial } : initial;
  // V107-N5a An icon-only control gets its tip as its name (screen readers, tests).
  const named = !node.getAttribute('aria-label') && !node.textContent?.trim();
  const name = () => named && opts.label && node.setAttribute('aria-label', opts.label);
  name();
  const enter = () => show(node, opts);
  // Keyboard focus only: a click already shows where the pointer is.
  const focus = (e: FocusEvent) => (e.target as Element).matches(':focus-visible') && focusTipAllowed(lastKey) && show(node, opts);
  const leave = () => owner === node && hide();
  node.addEventListener('mouseenter', enter);
  node.addEventListener('mouseleave', hide);
  node.addEventListener('mousedown', hide);
  node.addEventListener('focusin', focus);
  node.addEventListener('focusout', leave);
  return {
    update(next: TipOptions | string) {
      opts = typeof next === 'string' ? { label: next } : next;
      name();
      if (!tip.visible || owner !== node) return;
      if (!opts.label && !opts.image) return hide();
      tip.label = opts.label;
      tip.hint = hintFor(opts);
      tip.image = opts.image ?? '';
      tip.paint = opts.paint ?? '';
    },
    destroy() {
      if (owner === node) hide();
      node.removeEventListener('mouseenter', enter);
      node.removeEventListener('mouseleave', hide);
      node.removeEventListener('mousedown', hide);
      node.removeEventListener('focusin', focus);
      node.removeEventListener('focusout', leave);
    },
  };
}
