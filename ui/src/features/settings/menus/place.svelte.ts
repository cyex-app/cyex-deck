// V110-U67 Which menu Settings › Menus has open, for the rest of this run. A settings search
// that lands on a menu's entry (`menus:<id>`) opens that menu's editor.

export const menusView = $state({ id: '' });

export const ENTRY_PREFIX = 'menus:';

/** A settings search picked `entryId`: its menu opens (the page may not be mounted yet). */
export function openMenuFor(entryId: string): void {
  if (entryId.startsWith(ENTRY_PREFIX)) menusView.id = entryId.slice(ENTRY_PREFIX.length);
}

/** A page asks Settings to open another page (a link inside a page). SettingsView opens it and clears this. */
export const settingsJump = $state({ to: '' });
