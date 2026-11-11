import type { MenuEntry } from '../menuItems.ts';

// V110-N5 R2-6 The chat's Filters row. With no filters made it is the row itself, greyed out, with the
// hint as its note: a submenu of one disabled row can't be left with Left and traps Right / Enter.
export const FILTERS_HINT = 'Make filters in Settings → Filters';

export function filtersEntry(defs: { id: string; name: string }[], active: string[], flip: (id: string) => void): MenuEntry {
  if (!defs.length) return { label: 'Filters', icon: 'filter', disabled: true, note: FILTERS_HINT };
  return { label: 'Filters', icon: 'filter', items: defs.map((d) => ({ label: d.name, checked: active.includes(d.id), run: () => flip(d.id) })) };
}
