// V1051-03 The menu's paint classes live in one stylesheet, adopted on open and dropped on close.

import type { PaintStyle } from '../../ipc/bindings/PaintStyle';
import { paintRules, SAFE_ID } from '../emotes/looks.svelte';

export function menuSheet(paints: readonly PaintStyle[]): { adopt(): void; drop(): void } {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(paints.filter((p) => SAFE_ID.test(p.id)).map(paintRules).join(''));
  return {
    adopt() {
      if (!document.adoptedStyleSheets.includes(sheet)) document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
    },
    drop() {
      document.adoptedStyleSheets = document.adoptedStyleSheets.filter((s) => s !== sheet);
      sheet.replaceSync('');
    },
  };
}
