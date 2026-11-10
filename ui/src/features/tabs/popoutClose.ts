// V110-A-idea "Close all pop-outs" in the tab bar menu: every tab and chat that sits in its own
// window goes back to the main window. Pure, so the list can be tested.

export interface PopoutsOpen {
  tabs: ReadonlySet<string>;
  chats: ReadonlySet<string>;
}

export type PopoutRef = { kind: 'tab' | 'chat'; id: string };

/** The windows to close, tabs first. */
export function popoutRefs(open: PopoutsOpen): PopoutRef[] {
  return [...[...open.tabs].map((id): PopoutRef => ({ kind: 'tab', id })), ...[...open.chats].map((id): PopoutRef => ({ kind: 'chat', id }))];
}

/** The menu row's hint: how many, or why it's off. */
export function popoutHint(n: number): string {
  return n === 0 ? 'No pop-outs open.' : n === 1 ? 'Puts 1 window back in the main window.' : `Puts ${n} windows back in the main window.`;
}
