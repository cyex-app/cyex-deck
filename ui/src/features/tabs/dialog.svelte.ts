// Which add/edit dialog is open (1.16-1.18), shared by the tab bar, menus and switcher.

export type TabDialogMode =
  /** A new tab, or a new tile in the tab `into` (U3). `text` prefills the channel. */
  | { kind: 'add'; into?: string; text?: string }
  /** 1.18 A tile's channel, stream/chat and account, plus its tab's name. */
  | { kind: 'edit'; tab: string; tile: string };

export const tabDialog = $state({ mode: null as TabDialogMode | null });

export function openAdd(into?: string, text?: string): void {
  tabDialog.mode = { kind: 'add', into, text };
}

export function openEdit(tab: string, tile: string): void {
  tabDialog.mode = { kind: 'edit', tab, tile };
}

export function closeTabDialog(): void {
  tabDialog.mode = null;
}

/** U5 Name and color for a new group of `tabs`, or for an existing `group`. */
export type GroupDialogMode = { tabs: string[] } | { group: string };

export const groupDialog = $state({ mode: null as GroupDialogMode | null });

export function openGroupDialog(mode: GroupDialogMode): void {
  groupDialog.mode = mode;
}
