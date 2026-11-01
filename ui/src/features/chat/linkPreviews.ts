// V108-C25 The one queue every chat asks link cards through (1.52): hovers, cards under
// messages (C30) and lines coming in.

import type { LinkPreview } from '../../ipc/bindings/LinkPreview';
import { chatIpc } from '../../ipc/chat';
import { settings } from '../../lib/settings.svelte';
import { streamer } from '../../lib/streamer.svelte';
import { linkQueue } from './linkQueue';

export const links = linkQueue<LinkPreview | null>((url) => chatIpc.resolveLink(url));

/** Cards show: previews on and streamer mode not hiding them. */
export function cardsShown(): boolean {
  return !!settings.v.chat?.linkPreviews && !streamer.hides('hideLinkPreviews');
}

/** A drawn line's links, asked for ahead of a hover. */
export function prefetchLinks(urls: readonly string[]): void {
  if (!urls.length || !cardsShown()) return;
  for (const u of urls) links.prefetch(u);
}
