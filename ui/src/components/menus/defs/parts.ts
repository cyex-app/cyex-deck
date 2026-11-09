// V110-U66 Row shapes the tab, tile and chat menus share. Pure.

import { noop } from './make.ts';

/** A row that does something. */
export interface Act {
  run(): void;
}

/** Hide chat / Show chat, Hide stream / Show stream: the label says what it does now. */
export interface ShowRow extends Act {
  label: string;
}

/** An on/off row with its shortcut, e.g. Only live tabs, Lock tabs. */
export interface Switch extends Act {
  on: boolean;
  hint?: string;
}

/** V102-23 Moving a tab's panels and putting them back; `null` where the tab has nothing to move. */
export interface LayoutPart {
  /** Only when panels were moved. */
  reset: (() => void) | null;
  panels: Switch;
}


export const sampleLayout = (reset = true): LayoutPart => ({ reset: reset ? noop : null, panels: { on: false, run: noop } });
