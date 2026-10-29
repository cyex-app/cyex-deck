// V108-N12 Which chat views send to their own channel only (Shared Chat). `chatIpc.send` asks
// here, so the message box needs no change. Set by the chat header, cleared when it ends.

const views = new Set<number>();

export const sourceOnly = {
  set(view: number, on: boolean): void {
    if (!view) return;
    if (on) views.add(view);
    else views.delete(view);
  },
  has: (view: number): boolean => views.has(view),
};

/** A plain line: no command (`/` or Twitch's `.`), not a reply. Helix can't say "own channel
 *  only" for those (commands, `/me`) or for a reply here, so they go the normal way. */
export function plainLine(text: string, replyTo?: string): boolean {
  const t = text.trim();
  return !replyTo && t !== '' && !t.startsWith('/') && !t.startsWith('.');
}
