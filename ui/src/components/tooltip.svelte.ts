// Tooltips (3.6). One shared tip for the whole window. The label always shows; the
// explanation underneath is the "new user" part and follows the tooltips setting (2.7).

import type { LinkText } from '../ipc/bindings/LinkText';
import type { Rect, Side } from '../lib/place';
import { settings } from '../lib/settings.svelte';
import { tipAllowed } from './tooltipGate';

export interface TipOptions {
  label: string;
  hint?: string;
  /** E6 A bigger picture above the label, e.g. an emote's. */
  image?: string;
  /** Shows only when this says so at that moment, e.g. while a label is cut off. */
  when?: () => boolean;
  /** ct-9 Where it goes when there's room; below by default. */
  side?: Side;
}

/** 1.52 A link's preview under the label. */
export interface LinkPreview {
  lines: LinkText[][];
  thumbnail: string;
  /** 0 hides the picture. */
  thumbWidth: number;
}

export const tip = $state({
  label: '',
  hint: '',
  image: '',
  preview: null as LinkPreview | null,
  /** The element it shows for, as it was when the tip opened. */
  anchor: null as Rect | null,
  side: 'below' as Side,
  visible: false,
});

const SHOW_DELAY_MS = 150;
/** Moving between buttons right after a tip closed shows the next one instantly. */
const WARM_MS = 400;

let timer: ReturnType<typeof setTimeout> | undefined;
let lastHidden = 0;
/** The element the tip shows (or is about to show) for: only it may retitle it. */
let owner: HTMLElement | null = null;

/** For lists that delegate: one listener on the list instead of one per item. */
export function showTip(node: HTMLElement, opts: TipOptions): void {
  show(node, opts);
}

export function hideTip(): void {
  hide();
}

/** Changes the visible tip's text, e.g. once a link resolves. */
export function retitleTip(label: string): void {
  if (tip.visible) tip.label = label;
}

/** 1.52 Adds a link's preview to the tip `node` shows, once it's known. */
export function previewTip(node: HTMLElement, preview: LinkPreview): void {
  if (owner === node) tip.preview = preview;
}

function hintFor(opts: TipOptions): string {
  return opts.image ? (opts.hint ?? '') : settings.v.general?.tooltips ? (opts.hint ?? '') : '';
}

function show(node: HTMLElement, opts: TipOptions): void {
  clearTimeout(timer);
  // Nothing to say (e.g. a menu item without a note), or a drag is on (rb-1).
  if (!tipAllowed(opts, dragging())) return;
  owner = node;
  const delay = performance.now() - lastHidden < WARM_MS ? 0 : SHOW_DELAY_MS;
  timer = setTimeout(() => {
    // rb-1 A drag that started during the delay.
    if (dragging()) return;
    tip.label = opts.label;
    tip.hint = hintFor(opts);
    tip.image = opts.image ?? '';
    tip.preview = null;
    tip.anchor = node.getBoundingClientRect();
    tip.side = opts.side ?? 'below';
    tip.visible = true;
  }, delay);
}

/** rb-1 The shared drag (features/tabs/drag) marks the body while a drag is on. */
const dragging = () => typeof document !== 'undefined' && document.body.classList.contains('dragging');

function hide(): void {
  clearTimeout(timer);
  if (tip.visible) lastHidden = performance.now();
  tip.visible = false;
  owner = null;
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
}

/** `use:tooltip={{ label, hint }}` or `use:tooltip={'Label'}`. Shows on hover and on
 *  keyboard focus; `update` refreshes the visible tip. */
export function tooltip(node: HTMLElement, initial: TipOptions | string) {
  let opts = typeof initial === 'string' ? { label: initial } : initial;
  const enter = () => show(node, opts);
  // Keyboard focus only: a click already shows where the pointer is.
  const focus = (e: FocusEvent) => (e.target as Element).matches(':focus-visible') && show(node, opts);
  const leave = () => owner === node && hide();
  node.addEventListener('mouseenter', enter);
  node.addEventListener('mouseleave', hide);
  node.addEventListener('mousedown', hide);
  node.addEventListener('focusin', focus);
  node.addEventListener('focusout', leave);
  return {
    update(next: TipOptions | string) {
      opts = typeof next === 'string' ? { label: next } : next;
      if (!tip.visible || owner !== node) return;
      if (!opts.label && !opts.image) return hide();
      tip.label = opts.label;
      tip.hint = hintFor(opts);
      tip.image = opts.image ?? '';
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
