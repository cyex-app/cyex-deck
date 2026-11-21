// 1.57 A chatter's role from their badges, as Rust's `ModRole::of` reads them
// (crates/chat/src/model/role.rs). What a role may do is Rust's answer (`may.ts`).

import type { ModRole } from '../../ipc/bindings/ModRole';

/** V112-21 A badge set that says someone moderates: lead mods (head mods) are mods too. */
export const isModSet = (set: string): boolean => set === 'moderator' || set === 'lead_moderator';

/** A chatter's role, from their badges. */
export function roleOf(badges: readonly { set: string }[] | undefined): ModRole | null {
  if (!badges) return null;
  const has = (set: string) => badges.some((b) => b.set === set);
  if (has('broadcaster')) return 'broadcaster';
  if (has('lead_moderator')) return 'leadModerator';
  if (has('moderator')) return 'moderator';
  return null;
}
