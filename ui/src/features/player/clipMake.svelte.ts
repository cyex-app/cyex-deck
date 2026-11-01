// V108-N01, N25 Clips made from a place that isn't the live edge: a VOD at the current time,
// or a moment in the stream. Always a click on the user's side; the new clip plays over the
// player, like the clip button's (P9) or beside the chat of a chat-only tile.

import { playerIpc } from '../../ipc/player';
import { errorText } from '../../lib/errors';
import { toast } from '../../lib/toast.svelte';
import { markFresh } from './freshClips';
import { openTarget } from './players.svelte';

/** A clip is being made: one at a time, Twitch throttles more anyway. */
let busy = $state(false);

export const clipMaking = {
  get busy(): boolean {
    return busy;
  },
  /** V108-P2 The live clip button (P9) takes the same single slot. False when one is running. */
  begin(): boolean {
    if (busy) return false;
    busy = true;
    return true;
  },
  end(): void {
    busy = false;
  },
};

type Where = Parameters<typeof openTarget>[0];

async function make(tile: Where, ask: () => Promise<{ slug: string }>): Promise<void> {
  if (busy) return;
  busy = true;
  toast.info('Making a clip…');
  try {
    const clip = await ask();
    markFresh(clip.slug);
    // Over the stream, or beside the chat of a chat-only tile.
    openTarget(tile, { kind: 'clip', slug: clip.slug });
  } catch (e) {
    toast.error(errorText(e));
  } finally {
    busy = false;
  }
}

/** The live stream up to `offset` s into the broadcast. */
export function clipLiveAt(tile: Where, account: string, channelId: string, offset: number): Promise<void> {
  return make(tile, () => playerIpc.createClipAt(account, channelId, offset));
}

/** The video up to `offset` s into it. */
export function clipVideoAt(tile: Where, account: string, channelId: string, videoId: string, offset: number): Promise<void> {
  return make(tile, () => playerIpc.createClipFromVideo(account, channelId, videoId, offset));
}
