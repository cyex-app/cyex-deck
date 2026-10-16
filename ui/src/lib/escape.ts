// Layers that close on Escape: dialogs, popovers, menus, open selects. One Escape closes
// only the topmost one. The stack listens on the window in the capture phase, before any
// other key handler, and swallows the key it uses (preventDefault and
// stopImmediatePropagation), so nothing behind the layer reacts to it.
//
// A field that clears itself on Escape marks itself `data-own-escape` while it has text;
// the stack then leaves that Escape to the field (SearchField does this).
//
//   const off = layer('dialog', onclose);   // on open
//   off();                                  // on close (safe to call twice)

export type LayerKind = 'dialog' | 'popover' | 'menu' | 'select';

export interface Layer {
  kind: LayerKind;
  close: () => void;
}

/** The parts of a KeyboardEvent the stack reads; a plain object in tests. */
export interface EscapeEvent {
  key: string;
  defaultPrevented: boolean;
  isComposing?: boolean;
  target: EventTarget | null;
  preventDefault(): void;
  stopImmediatePropagation(): void;
}

/** Layers that take the keyboard: app shortcuts and player keys wait while one is open. */
const BLOCKING: ReadonlySet<LayerKind> = new Set(['dialog', 'menu', 'select']);

export class LayerStack {
  private layers: Layer[] = [];

  /** Adds a layer on top. The returned function removes it, wherever it is by then. */
  push(layer: Layer): () => void {
    this.layers.push(layer);
    return () => {
      this.layers = this.layers.filter((l) => l !== layer);
    };
  }

  get top(): Layer | undefined {
    return this.layers.at(-1);
  }

  get size(): number {
    return this.layers.length;
  }

  /** True while a dialog, menu or open select is up. */
  blocking(): boolean {
    return this.layers.some((l) => BLOCKING.has(l.kind));
  }

  /** True while the only blocking layers are menus: the quick switcher (Ctrl+K) may still open and close them. */
  onlyMenus(): boolean {
    const held = this.layers.filter((l) => BLOCKING.has(l.kind));
    return held.length > 0 && held.every((l) => l.kind === 'menu');
  }

  /** Closes the topmost layer on Escape. True when it did, and the key is then used up. */
  escape(e: EscapeEvent): boolean {
    if (e.key !== 'Escape' || e.defaultPrevented || e.isComposing) return false;
    const top = this.top;
    if (!top || ownsEscape(e.target)) return false;
    e.preventDefault();
    e.stopImmediatePropagation();
    // The layer stays until its owner removes it, so one that refuses to close (a running
    // import) still holds Escape.
    top.close();
    return true;
  }
}

function ownsEscape(target: EventTarget | null): boolean {
  const el = target as { closest?: (s: string) => unknown } | null;
  return typeof el?.closest === 'function' && !!el.closest('[data-own-escape]');
}

export const layers = new LayerStack();

let listening = false;

/** Opens a layer; call the result when it closes. */
export function layer(kind: LayerKind, close: () => void): () => void {
  if (!listening && typeof window !== 'undefined') {
    window.addEventListener('keydown', (e) => layers.escape(e), true);
    listening = true;
  }
  return layers.push({ kind, close });
}

/** For global key handlers: true when the key belongs to an open layer or was already used. */
export function keyTaken(e: { defaultPrevented: boolean }): boolean {
  return e.defaultPrevented || layers.blocking();
}
