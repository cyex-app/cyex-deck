// Tooltips (3.6). One shared tip for the whole window. The label always shows; the
// explanation underneath is the "new user" part and follows the tooltips setting (2.7).

import { settings } from '../lib/settings.svelte';

export interface TipOptions {
  label: string;
  hint?: string;
}

export const tip = $state({ label: '', hint: '', x: 0, y: 0, visible: false });

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

function show(node: HTMLElement, opts: TipOptions): void {
  clearTimeout(timer);
  owner = node;
  const delay = performance.now() - lastHidden < WARM_MS ? 0 : SHOW_DELAY_MS;
  timer = setTimeout(() => {
    const r = node.getBoundingClientRect();
    tip.label = opts.label;
    tip.hint = settings.v.general?.tooltips ? (opts.hint ?? '') : '';
    tip.x = r.left + r.width / 2;
    tip.y = r.bottom + 6;
    tip.visible = true;
  }, delay);
}

function hide(): void {
  clearTimeout(timer);
  if (tip.visible) lastHidden = performance.now();
  tip.visible = false;
  owner = null;
}

/** `use:tooltip={{ label, hint }}` or `use:tooltip={'Label'}`. */
export function tooltip(node: HTMLElement, initial: TipOptions | string) {
  let opts = typeof initial === 'string' ? { label: initial } : initial;
  const enter = () => show(node, opts);
  node.addEventListener('mouseenter', enter);
  node.addEventListener('mouseleave', hide);
  node.addEventListener('mousedown', hide);
  return {
    update(next: TipOptions | string) {
      opts = typeof next === 'string' ? { label: next } : next;
      if (tip.visible && owner === node) tip.label = opts.label;
    },
    destroy() {
      if (owner === node) hide();
      node.removeEventListener('mouseenter', enter);
      node.removeEventListener('mouseleave', hide);
      node.removeEventListener('mousedown', hide);
    },
  };
}
