// The stream manager's panels, in page order (never changes; the chat is last, so stacked it's
// at the bottom). V109-C04 Followers, rewards, clips, videos and moderation settings moved to
// the creator dashboard: a layout saved with them still loads, without them.

import { type Node, prune } from '../../lib/panels.ts';

export const SM_PANELS = [
  'stream',
  'info',
  'modes',
  'activity',
  'actions',
  'poll',
  'prediction',
  'raid',
  'ads',
  'markers',
  'together',
  'roles',
  'rolesmgr',
  'chat',
] as const;

export type SmPanel = (typeof SM_PANELS)[number];

const known = (id: string): id is SmPanel => (SM_PANELS as readonly string[]).includes(id);

/** A saved layout with only the panels the manager still has; null when none are left. */
export function savedLayout(saved: Node | null | undefined): Node | null {
  return prune(saved ?? null, known);
}
