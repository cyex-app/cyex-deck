// Tooltips (3.6). One shared tip for the whole window. The label always shows; the
// explanation underneath is the "new user" part and follows the tooltips setting (2.7).

import type { LinkText } from '../ipc/bindings/LinkText';
import type { Rect } from '../lib/place';
import { settings } from '../lib/settings.svelte';

export interface TipOptions {
  label: string;
  hint?: string;
  /** E6 A bigger picture above the label, e.g. an emote's. */
  image?: string;
  /** Shows only when this says so at that moment, e.g. while a label is cut off. */
  when?: () => boolean;
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
  // Nothing to say, e.g. a menu item without a note.
  if ((!opts.label && !opts.image) || (opts.when && !opts.when())) return;
  owner = node;
  const delay = performance.now() - lastHidden < WARM_MS ? 0 : SHOW_DELAY_MS;
  timer = setTimeout(() => {
    tip.label = opts.label;
    tip.hint = hintFor(opts);
    tip.image = opts.image ?? '';
    tip.preview = null;
    tip.anchor = node.getBoundingClientRect();
    tip.visible = true;
  }, delay);
}

function hide(): void {
  clearTimeout(timer);
  if (tip.visible) lastHidden = performance.now();
  tip.visible = false;
  owner = null;
}

// A scroll that moves the tip's element hides it, and so does Escape. Added at load, before
// the Escape layer stack (lib/escape.ts) listens, which uses the key up when it closes a layer.
if (typeof window !== 'undefined') {
  const moved = () => {
    if (!tip.visible || !owner || !tip.anchor) return;
    const r = owner.getBoundingClientRect();
    if (Math.abs(r.top - tip.anchor.top) > 1 || Math.abs(r.left - tip.anchor.left) > 1) hide();
  };
  addEventListener('scroll', moved, { capture: true, passive: true });
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
