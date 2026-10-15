// V1051-04 The combos strip's state for one open menu. It talks to the panel's store
// (the shown pair, the one message and the one Undo) and never imports the panel.

import type { Combo, ComboItem, SelectError } from '../../ipc/cosmetics';
import { cosmeticsIpc } from '../../ipc/cosmetics';
import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';
import { current, problems, type Owned, type Problem } from './combos';
import { errorText, type CosmeticsStore } from './cosmetics.svelte';
import type { Pair } from './select';

export const MAX_COMBOS = 50;
const asError = (e: unknown): SelectError => (e && typeof e === 'object' && 'type' in e ? (e as SelectError) : { type: 'down' });

export class ComboStore {
  private readonly store: CosmeticsStore;
  readonly account: string;
  /** The combo being edited and its draft pair. */
  editing = $state.raw<Combo | null>(null);
  draft = $state.raw<Pair | null>(null);
  /** The Save card is a text field. */
  naming = $state(false);
  /** The card whose name is a text field. */
  renaming = $state<string | null>(null);
  /** A card to focus and scroll to once it exists. */
  focusId = $state<string | null>(null);

  constructor(store: CosmeticsStore) {
    this.store = store;
    this.account = store.account;
  }

  get list(): readonly Combo[] {
    return settings.v.combos?.accounts?.[this.account] ?? [];
  }
  get full(): boolean {
    return this.list.length >= MAX_COMBOS;
  }
  /** What the grids and the preview show: the draft in edit mode. */
  get shown(): Pair {
    return this.draft ?? this.store.shown;
  }
  get worn(): Combo | null {
    return current(this.list, this.store.shown.paint, this.store.shown.badge);
  }

  have = (kind: 'paint' | 'badge', id: string): Owned => {
    const item = kind === 'paint' ? this.store.paintById.get(id) : this.store.badgeById.get(id);
    return !item ? 'notOwned' : item.usable ? 'usable' : 'needsSub';
  };
  problemsOf = (c: Combo): Problem[] => (this.store.view ? problems(c, this.have) : []);

  /** The shown pair with the names the view has. */
  private item(kind: 'paint' | 'badge', id: string | null): ComboItem | null {
    if (!id) return null;
    const keep = this.editing?.[kind];
    if (keep?.id === id) return keep;
    const name = kind === 'paint' ? this.store.paintById.get(id)?.style.name : this.store.badgeById.get(id)?.def.title;
    return { id, name: name ?? id };
  }
  private pair(p: Pair): { paint: ComboItem | null; badge: ComboItem | null } {
    return { paint: this.item('paint', p.paint), badge: this.item('badge', p.badge) };
  }

  async apply(c: Combo) {
    if (this.editing) return;
    const st = this.store.status;
    if (st === 'signedOut' || st === 'expired') {
      this.store.say(st === 'expired' ? 'Sign in again to put it on.' : 'Sign in to 7TV to put it on.');
      return;
    }
    const before: Pair = { ...this.store.sel.confirmed };
    this.store.sel.adopt({ paint: c.paint?.id ?? null, badge: c.badge?.id ?? null });
    this.store.say('Saving…');
    try {
      const a = await cosmeticsIpc.apply(this.account, c.paint, c.badge, c.name);
      this.store.sel.adopt({ paint: a.nowPaint, badge: a.nowBadge });
      this.store.say(a.line, () => void this.restoreWorn(before));
    } catch (e) {
      this.store.sel.adopt(before);
      this.store.say(errorText(asError(e)));
      if (asError(e).type !== 'down') void this.store.load();
    }
  }

  /** Undo of an apply: the previous pair, nothing saved as a combo. */
  private async restoreWorn(p: Pair) {
    const was: Pair = { ...this.store.sel.confirmed };
    const w = this.pair(p);
    this.store.sel.adopt(p);
    this.store.say('Saving…');
    try {
      const a = await cosmeticsIpc.apply(this.account, w.paint, w.badge, null);
      this.store.sel.adopt({ paint: a.nowPaint, badge: a.nowBadge });
      this.store.say('Done.', null, true);
    } catch (e) {
      this.store.sel.adopt(was);
      this.store.say(errorText(asError(e)));
    }
  }

  /** Runs one change; the message on failure goes to the caller. */
  private async change(op: Parameters<typeof cosmeticsIpc.combosChange>[1]) {
    return cosmeticsIpc.combosChange(this.account, op);
  }

  /** Returns Rust's error text, or null when it worked. */
  async save(name: string): Promise<string | null> {
    const p = this.pair(this.store.shown);
    try {
      const r = await this.change({ type: 'save', name, ...p });
      if (r.type === 'saved') this.focusId = r.combo.id;
      return null;
    } catch (e) {
      return String(e);
    }
  }

  async rename(c: Combo, name: string): Promise<string | null> {
    try {
      await this.change({ type: 'rename', id: c.id, name });
      return null;
    } catch (e) {
      return String(e);
    }
  }

  async duplicate(c: Combo) {
    try {
      const r = await this.change({ type: 'duplicate', id: c.id });
      if (r.type === 'saved' || r.type === 'changed') this.focusId = r.combo.id;
    } catch (e) {
      toast.error(String(e));
    }
  }

  async remove(c: Combo) {
    try {
      const r = await this.change({ type: 'delete', id: c.id });
      if (r.type !== 'deleted') return;
      const { combo, at } = r;
      this.store.say(`Deleted “${combo.name}”.`, () => void this.change({ type: 'restore', combo, at }).then(
        (back) => {
          if (back.type !== 'deleted') this.focusId = back.combo.id;
          this.store.say('Done.', null, true);
        },
        (err) => toast.error(String(err)),
      ));
    } catch (e) {
      toast.error(String(e));
    }
  }

  edit(c: Combo) {
    this.editing = c;
    this.draft = { paint: c.paint?.id ?? null, badge: c.badge?.id ?? null };
  }
  pick(kind: 'paint' | 'badge', id: string | null) {
    if (this.draft) this.draft = { ...this.draft, [kind]: id };
  }
  async commit() {
    const c = this.editing;
    if (!c || !this.draft) return;
    try {
      await this.change({ type: 'edit', id: c.id, ...this.pair(this.draft) });
      this.cancel();
    } catch (e) {
      toast.error(String(e));
    }
  }
  cancel() {
    this.editing = null;
    this.draft = null;
  }
}
