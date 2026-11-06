// V108-N06 The extra badge sets (FFZ:AP, Chatsen, DankChat, Homies). Their badges arrive with
// `provider: "twitch"` and an id like `ffzap:123`; the prefix says which switch governs them.

import type { BadgeSets } from '../../ipc/bindings/BadgeSets.ts';

export const BADGE_SETS: { key: keyof BadgeSets; label: string; hint: string }[] = [
  { key: 'ffzap', label: 'FFZ:AP', hint: 'Supporters of the FFZ add-on pack.' },
  { key: 'chatsen', label: 'Chatsen', hint: 'People behind the Chatsen app and its supporters.' },
  { key: 'dankchat', label: 'DankChat', hint: 'People behind the DankChat app and its supporters.' },
  { key: 'homies', label: 'Homies', hint: 'Chatterino Homies, with the badges people made for themselves.' },
];

/** The set a provider badge id belongs to; null for the four built-in providers. */
export function setOf(id: string): keyof BadgeSets | null {
  const at = id.indexOf(':');
  if (at < 1) return null;
  const key = id.slice(0, at);
  return BADGE_SETS.some((s) => s.key === key) ? (key as keyof BadgeSets) : null;
}

/** Whether a provider badge is switched on: its set's switch, or its provider's. V109-B02 cyex
 * badges (`cyex:…`) have their own switch beside the providers' (`chat.badges.cyex`). */
export function badgeOn(
  b: { provider: string; id: string },
  providers: Record<string, boolean>,
  sets: BadgeSets,
): boolean {
  if (b.id.startsWith('cyex:')) return !!providers.cyex;
  const set = setOf(b.id);
  return set ? sets[set] : !!providers[b.provider];
}
