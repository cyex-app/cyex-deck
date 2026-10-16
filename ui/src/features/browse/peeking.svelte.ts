// 1.6 Hovering a live channel in a list shows its preview picture and title.

import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
import type { Rect } from '../../lib/place';

const DELAY_MS = 300;

export const peek = $state({ card: null as ChannelCard | null, anchor: null as Rect | null });

let timer: ReturnType<typeof setTimeout> | undefined;
let owner: HTMLElement | null = null;

export function peekAt(node: HTMLElement, card: ChannelCard): void {
  clearTimeout(timer);
  owner = node;
  timer = setTimeout(() => {
    if (!node.isConnected) return;
    peek.anchor = node.getBoundingClientRect();
    peek.card = card;
  }, DELAY_MS);
}

export function unpeek(): void {
  clearTimeout(timer);
  owner = null;
  peek.card = null;
}

/** A row that goes away under the pointer (account switch, refresh) gets no mouseleave: it
 *  takes its peek with it. */
export function unpeekFrom(node: HTMLElement | undefined): void {
  if (node && owner === node) unpeek();
}
