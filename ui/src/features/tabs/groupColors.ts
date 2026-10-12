// U5 Group colors: mid tones that read on both themes as dots and underlines.

import type { GroupColor } from '../../ipc/bindings/GroupColor';

export const GROUP_COLORS: Record<GroupColor, { css: string; name: string }> = {
  grey: { css: '#8b8d96', name: 'Grey' },
  blue: { css: '#4d8dff', name: 'Blue' },
  red: { css: '#ef5b5b', name: 'Red' },
  yellow: { css: '#e8a93a', name: 'Yellow' },
  green: { css: '#2fbf7f', name: 'Green' },
  pink: { css: '#ea6aaa', name: 'Pink' },
  purple: { css: '#9d6bff', name: 'Purple' },
  cyan: { css: '#1fb5c9', name: 'Cyan' },
};

export const COLOR_ORDER = Object.keys(GROUP_COLORS) as GroupColor[];

/** The first color no group uses yet, so new groups look different. */
export function nextColor(used: GroupColor[]): GroupColor {
  return COLOR_ORDER.find((c) => c !== 'grey' && !used.includes(c)) ?? 'grey';
}
