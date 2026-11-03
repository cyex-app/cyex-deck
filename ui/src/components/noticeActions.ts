// V108-C39 The button on a live card: "Go to tab" when the channel has one, else "Open tab".
// Decided from the tabs as they are when the card is drawn, so it is right even if a tab was
// opened or closed while the card was up.

import type { NotifyOpen } from '../ipc/bindings/NotifyOpen';

/** A tab and the channel logins in it. */
export interface TabRef {
  id: string;
  logins: string[];
}

export interface CardAction {
  label: 'Open tab' | 'Go to tab';
  /** What pressing it asks the main window to do. */
  open: NotifyOpen;
  /** Whether the window must come to the front first. */
  raise: boolean;
}

/** The tab that shows `login`: one that shows only it first, else any that has it. */
export function tabHolding(tabs: readonly TabRef[], login: string): string | null {
  const l = login.toLowerCase();
  const has = tabs.filter((t) => t.logins.some((x) => x.toLowerCase() === l));
  return (has.find((t) => t.logins.length === 1) ?? has[0])?.id ?? null;
}

/** The button for a card, or none (only live cards about a channel have one). */
export function cardAction(kind: string, open: NotifyOpen, tabs: readonly TabRef[]): CardAction | null {
  if (kind !== 'live' || open.kind !== 'channel') return null;
  if (tabHolding(tabs, open.login))
    return { label: 'Go to tab', open: { kind: 'tab', login: open.login }, raise: true };
  return { label: 'Open tab', open: { kind: 'background', login: open.login }, raise: false };
}
