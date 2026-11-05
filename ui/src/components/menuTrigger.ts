// V109-F61 The button a shared menu opened from. Clicking it again closes the menu: the
// outside-click handler leaves the button alone (otherwise it closes the menu on mouse-down
// and the click opens it again), and `openMenu` shuts the menu when its own button asks.
// The button also says `aria-expanded` while its menu is open.

export interface TriggerEl {
  isConnected: boolean;
  contains(node: unknown): boolean;
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
  removeAttribute(name: string): void;
}

export interface MenuTrigger {
  /** The menu opens from `el` (null: from the pointer, no button). */
  set(el: TriggerEl | null): void;
  /** `el` is the button of the menu that is open now: the click should close it. */
  isOpenFor(el: unknown): boolean;
  /** `node` is the button or inside it: the outside-click handler skips it. */
  owns(node: unknown): boolean;
  clear(): void;
}

export function menuTrigger(): MenuTrigger {
  let el: TriggerEl | null = null;
  /** What `aria-expanded` said before the menu took it over (a button may manage its own). */
  let before: string | null = null;
  const live = () => (el?.isConnected ? el : null);
  function clear() {
    if (before === null) el?.removeAttribute('aria-expanded');
    else el?.setAttribute('aria-expanded', before);
    el = null;
  }
  return {
    set(next) {
      clear();
      el = next;
      before = next?.getAttribute('aria-expanded') ?? null;
      el?.setAttribute('aria-expanded', 'true');
    },
    isOpenFor: (at) => !!el && live() === at,
    owns: (node) => !!live()?.contains(node),
    clear,
  };
}
