// 1.6 Hovering a live channel in a list shows its preview picture and title.

import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
import type { Rect } from '../../lib/place';

const DELAY_MS = 300;

export const peek = $state({ card: null as ChannelCard | null, anchor: null as Rect | null });

let timer: ReturnType<typeof setTimeout> | undefined;

export function peekAt(node: HTMLElement, card: ChannelCard): void {
  clearTimeout(timer);
  timer = setTimeout(() => {
    peek.anchor = node.getBoundingClientRect();
    peek.card = card;
  }, DELAY_MS);
}

export function unpeek(): void {
  clearTimeout(timer);
  peek.card = null;
}
