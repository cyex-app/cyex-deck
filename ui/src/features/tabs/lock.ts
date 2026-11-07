// V109-U05 Lock tabs: what the setting stops. Pure, so tests can import it.

type Item = { kind: string };
type Target = { kind: string };

/** A locked bar starts no drag of a tab. Tiles, panels and list rows aren't tabs. */
export function dragBlocked(locked: boolean, item: Item): boolean {
  return locked && item.kind === 'tab';
}

/** A tile dragged out into the bar (its own tab, or into another tab) would rearrange tabs. */
export function dropBlocked(locked: boolean, item: Item, target: Target): boolean {
  if (!locked) return false;
  if (item.kind === 'tab') return true;
  return item.kind === 'tile' && (target.kind === 'tab' || target.kind === 'bar' || target.kind === 'group');
}
