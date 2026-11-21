// V112-12 Which paint a name wears when a person has one from 7TV and one from cyex.tv: the
// cyex.tv one wins while its switch is on, else the 7TV one while that switch is on.
// V112-23 cyex.tv's switch counts only once cyex.tv is ready (`CYEXTV_READY`).

import type { Look } from '../../ipc/bindings/Look.ts';
import { cyextvOn } from '../../lib/cyextv.ts';

interface Switches {
  cyextvCosmetics: boolean;
  paints: { seventv: boolean };
}

export function wornPaint(look: Look | null | undefined, c: Switches, ready?: boolean): string | null {
  if (cyextvOn(c.cyextvCosmetics, ready) && look?.cyextvPaint) return look.cyextvPaint;
  return c.paints.seventv ? (look?.paint ?? null) : null;
}
