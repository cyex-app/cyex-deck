// 1.6 Hovering a live channel in a list shows its preview picture and title.

import type { ChannelCard } from '../../ipc/bindings/ChannelCard';

const DELAY_MS = 300;
const HEIGHT = 250;

export const peek = $state({ card: null as ChannelCard | null, x: 0, y: 0 });

let timer: ReturnType<typeof setTimeout> | undefined;

export function peekAt(node: HTMLElement, card: ChannelCard): void {
  clearTimeout(timer);
  timer = setTimeout(() => {
    const r = node.getBoundingClientRect();
    peek.x = Math.min(r.left + 48, innerWidth - 336);
    peek.y = r.bottom + 4 + HEIGHT > innerHeight ? r.top - 4 - HEIGHT : r.bottom + 4;
    peek.card = card;
  }, DELAY_MS);
}

export function unpeek(): void {
  clearTimeout(timer);
  peek.card = null;
}
