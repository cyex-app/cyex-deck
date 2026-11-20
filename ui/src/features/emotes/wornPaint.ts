// V112-12 Which paint a name wears when a person has one from 7TV and one from cyex.tv: the
// cyex.tv one wins while its switch is on, else the 7TV one while that switch is on.

import type { Look } from '../../ipc/bindings/Look.ts';

interface Switches {
  cyextvCosmetics: boolean;
  paints: { seventv: boolean };
}

export function wornPaint(look: Look | null | undefined, c: Switches): string | null {
  if (c.cyextvCosmetics && look?.cyextvPaint) return look.cyextvPaint;
  return c.paints.seventv ? (look?.paint ?? null) : null;
}
