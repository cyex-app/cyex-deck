// V106-30 Pure rule for the emote card's "show unlisted" button (no store imports, so tests can load it).

import type { UnlistedEmotes } from '../../ipc/bindings/UnlistedEmotes';

/** The card's big button: only while blur is on and the user never chose. */
export function unlistedPrompt(chat: { unlistedEmotes: UnlistedEmotes; unlistedChosen: boolean }, unlisted: boolean): boolean {
  return unlisted && chat.unlistedEmotes === 'blur' && !chat.unlistedChosen;
}
