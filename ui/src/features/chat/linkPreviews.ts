// V108-C25 The one queue every chat asks link cards through (1.52): hovers, cards under
// messages (C30). V108-F46 Nothing is asked ahead of a hover, except V110-C66 links on the
// trusted list (`embeds`): the resolver only sees links you hover or trusted ones in view.

import type { LinkPreview } from '../../ipc/bindings/LinkPreview';
import { chatIpc } from '../../ipc/chat';
import { settings } from '../../lib/settings.svelte';
import { streamer } from '../../lib/streamer.svelte';
import { linkQueue } from './linkQueue';

export const links = linkQueue<LinkPreview | null>((url) => chatIpc.resolveLink(url));

/** V110-C66 Cards under messages and asks ahead of a hover: trusted links only (Rust refuses
 *  the rest), three at a time. Rust keeps the answer, so the hover that follows is instant. */
export const embeds = linkQueue<LinkPreview | null>((url) => chatIpc.embedLink(url), 3);

/** Cards show: previews on and streamer mode not hiding them. */
export function cardsShown(): boolean {
  return !!settings.v.chat?.linkPreviews && !streamer.hides('hideLinkPreviews');
}
