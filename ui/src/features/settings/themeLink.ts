// V109-T01 What a theme's cyex.app link means for the editor and the theme list. Mirrors
// `ThemeLink::update_available` in crates/settings/src/schema/theme_link.rs.

import type { CustomTheme } from '../../ipc/bindings/CustomTheme';
import type { ThemeCopy } from '../../ipc/bindings/ThemeCopy';
import type { ThemeLink } from '../../ipc/bindings/ThemeLink';

export const LINK_BASE = 'https://cyex.app/t/';

export const linkOf = (id: string) => `${LINK_BASE}${id}`;

/** The newer version to offer, or null. */
export function updateAvailable(link: ThemeLink | undefined): number | null {
  if (!link || !link.sourceId || link.removed) return null;
  const v = link.latestVersion;
  return v > link.sourceVersion && v !== link.ignoredVersion ? v : null;
}

/** "Update available · v4" */
export const updateLabel = (v: number) => `Update available · v${v}`;

/** One line in the Go back menu: "v3 · 2026/11/02", "· your changes" when it had them. */
export function copyLabel(c: ThemeCopy): string {
  const parts = [`v${c.version}`];
  if (c.date) parts.push(c.date);
  if (c.edited) parts.push('your changes');
  return parts.join(' · ');
}

export type ShareState =
  /** Not shared: Share. */
  | { kind: 'none' }
  /** Shared: Copy link, Update (when changed), Stop sharing. */
  | { kind: 'shared'; id: string; link: string; version: number; changed: boolean };

export function shareState(t: CustomTheme): ShareState {
  const l = t.link;
  if (!l?.sharedId) return { kind: 'none' };
  return { kind: 'shared', id: l.sharedId, link: linkOf(l.sharedId), version: l.sharedVersion, changed: l.sharedChanged };
}

/** A save from the editor keeps the link as Rust last wrote it: the editor never changes it,
 *  and its copy may be older than an update check that ran meanwhile. */
export function withLink(next: CustomTheme, saved: readonly CustomTheme[]): CustomTheme {
  const link = saved.find((t) => t.id === next.id)?.link;
  const out = { ...next };
  if (link) out.link = link;
  else delete out.link;
  return out;
}

/** Report reasons go up to 500 characters. */
export const MAX_REASON = 500;
