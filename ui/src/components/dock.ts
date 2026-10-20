// V107-R29 The closed-panels dock: one icon per closed panel that has one, always shown;
// the rest wait in the "Closed panels" chip while unlocked. A click opens that panel's
// quick menu (one at a time); a press that turned into a drag doesn't.

export interface DockSplit<T extends { icon?: unknown }> {
  /** Their own icon in the row. */
  icons: T[];
  /** The chip's list, only while unlocked. */
  rest: T[];
}

export function splitDock<T extends { icon?: unknown }>(items: T[]): DockSplit<T> {
  return { icons: items.filter((i) => i.icon), rest: items.filter((i) => !i.icon) };
}

/** The quick menu open after a click on `id`'s icon. */
export function clicked(open: string | null, id: string, dragged: boolean): string | null {
  if (dragged) return null;
  return open === id ? null : id;
}

/** The open menu stays only while its panel is still in the dock. */
export function stillOpen(open: string | null, ids: string[]): string | null {
  return open !== null && ids.includes(open) ? open : null;
}
