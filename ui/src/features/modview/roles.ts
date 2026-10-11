// 1.57, V102-21 The one role check in the UI. The account's role comes from Rust (`mods`
// store); these mirror Rust's `Need::allows` (crates/moderation/src/command.rs) and
// `ModRole::of` (crates/chat/src/model/role.rs), so both sides agree.

import type { ModRole } from '../../ipc/bindings/ModRole';

/** What an action needs: any mod, a lead mod or the broadcaster (mod and VIP changes), or
 * the broadcaster. */
export type Need = 'mod' | 'lead' | 'broadcaster';

const RANK: Record<ModRole, number> = { moderator: 0, leadModerator: 1, broadcaster: 2 };
const LEAST: Record<Need, number> = { mod: 0, lead: 1, broadcaster: 2 };

export function allows(role: ModRole | null | undefined, need: Need): boolean {
  return !!role && RANK[role] >= LEAST[need];
}

/** A chatter's role, from their badges. */
export function roleOf(badges: readonly { set: string }[] | undefined): ModRole | null {
  if (!badges) return null;
  const has = (set: string) => badges.some((b) => b.set === set);
  if (has('broadcaster')) return 'broadcaster';
  if (has('lead_moderator')) return 'leadModerator';
  if (has('moderator')) return 'moderator';
  return null;
}
