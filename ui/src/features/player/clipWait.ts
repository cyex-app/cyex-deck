// V109-F73 What the clip player does after each ask for a clip: play it, ask again, or say why
// not. A clip still being made isn't gone (it was called gone after 30 s), and one Twitch
// couldn't make says so at once (it showed "Getting the clip ready…" for 30 s first).

export type Answer = 'ready' | 'making' | 'failed' | 'gone' | 'error';

/** Between asks. */
export const ASK_MS = 3000;
/** Asks while Twitch says it's making the clip: 2 min for one made here, 1 min for others. */
export const MAKING_FRESH = 40;
export const MAKING_OTHER = 20;
/** A clip made here may not be listed for a moment, and may answer oddly until it's done. */
export const GONE_FRESH = 5;
export const ERROR_FRESH = 10;
/** After this many asks the wait says it's slow. */
const SLOW = 10;

export const READY_TEXT = 'Getting the clip ready…';
export const SLOW_TEXT = 'Twitch is still making the clip…';
export const UNFINISHED_TEXT = 'Twitch hasn’t finished this clip yet. Try again in a while.';
export const FAILED_TEXT = 'Twitch couldn’t make this clip.';
export const GONE_TEXT = 'This clip is gone.';

/** `tries`: asks so far, this one included. `retry`: ask again; `text`: what shows (null for
 *  an error the caller words). */
export function afterAsk(answer: Answer, tries: number, fresh: boolean): { retry: boolean; text: string | null } {
  switch (answer) {
    case 'ready':
      return { retry: false, text: null };
    case 'making':
      if (tries < (fresh ? MAKING_FRESH : MAKING_OTHER)) return { retry: true, text: tries >= SLOW ? SLOW_TEXT : READY_TEXT };
      return { retry: false, text: UNFINISHED_TEXT };
    case 'failed':
      return { retry: false, text: FAILED_TEXT };
    case 'gone':
      return fresh && tries < GONE_FRESH ? { retry: true, text: READY_TEXT } : { retry: false, text: GONE_TEXT };
    case 'error':
      return fresh && tries < ERROR_FRESH ? { retry: true, text: READY_TEXT } : { retry: false, text: null };
  }
}
