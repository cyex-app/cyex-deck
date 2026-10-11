// V101-33/34 What a mod gets on chat lines: the pin button, and in mod mode delete, timeout
// and ban in front (1.57: only where the account moderates).

export interface LineMod {
  /** The account's user id: its own lines get no buttons. */
  me: string | null;
  broadcaster: boolean;
  /** Mod mode is on in this chat. */
  inline: boolean;
  /** The shortest timeout preset (2.11), if any. */
  timeout: number | null;
}

interface Who {
  id: string;
  login: string;
  badges: { set: string }[];
}

/** Mod mode's buttons go on chat lines from people a mod can act on: not the broadcaster,
 * not the account itself, and not other mods unless the account is the broadcaster. */
export function modButtonsFor(mod: LineMod | null, sender: Who | null | undefined, channel: string): boolean {
  if (!mod?.inline || !sender || sender.id === mod.me || sender.login === channel) return false;
  return mod.broadcaster || !sender.badges.some((b) => b.set === 'moderator' || b.set === 'lead_moderator');
}
