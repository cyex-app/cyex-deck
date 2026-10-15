// V1051-02/03 The state of one open cosmetics menu. One store per panel, so closing the
// panel frees all of it: the inventory, its stylesheet and its listeners.

import type { BadgeItem } from '../../ipc/bindings/BadgeItem';
import type { PaintItem } from '../../ipc/bindings/PaintItem';
import type { CosmeticsView, Kind, SelectError } from '../../ipc/cosmetics';
import { cosmeticsIpc } from '../../ipc/cosmetics';
import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { accounts } from '../../lib/accounts.svelte';
import { toast } from '../../lib/toast.svelte';
import { emptyQuery, filterItems, type Query } from './filter';
import { Selection, UNDO_MS } from './select';
import { menuSheet } from './sheet';

const toPaint = (item: PaintItem) => ({
  name: item.style.name,
  animated: item.animated,
  usable: item.usable,
  added: item.added,
  item,
});
const toBadge = (item: BadgeItem) => ({
  name: item.def.title,
  animated: item.animated,
  usable: item.usable,
  added: item.added,
  item,
});
export type PaintRow = ReturnType<typeof toPaint>;
export type BadgeRow = ReturnType<typeof toBadge>;

export function errorText(e: SelectError): string {
  switch (e.type) {
    case 'signInAgain':
      return 'Your 7TV sign-in ran out.';
    case 'notAllowed':
      return '7TV didn’t allow that one.';
    case 'rateLimited':
      return `7TV needs a short break. Try again in ${e.seconds} s.`;
    case 'down':
      return 'Couldn’t reach 7TV. Nothing changed.';
    default:
      return 'Sign in to 7TV first.';
  }
}

/** A sample line in the chosen theme. Name colors fill both slots so the box wins over the app's theme. */
export function previewMessage(who: { userId: string; login: string; displayName: string }, theme: 'dark' | 'light'): ChatMessage {
  const c = theme === 'dark' ? '#e8e8ea' : '#1b1c20';
  return {
    id: `local-preview-${theme}`,
    ts: Date.now(),
    sender: { id: who.userId, login: who.login, display: who.displayName, color: { dark: c, light: c }, badges: [] },
    body: [{ t: 'text', text: 'This is how you look in chat.' }],
    flags: [],
  };
}

export class CosmeticsStore {
  readonly account: string;
  view = $state.raw<CosmeticsView | null>(null);
  loading = $state(true);
  failed = $state(false);
  paintsQ = $state.raw<Query>(emptyQuery());
  badgesQ = $state.raw<Query>(emptyQuery());
  /** Bumped whenever the selection changes (it isn't reactive itself). */
  private tick = $state(0);
  /** Which account key the view was loaded for, to spot sign-ins and -outs elsewhere. */
  key = '';
  private readonly now = Date.now();
  private sheet: ReturnType<typeof menuSheet> | null = null;
  private handled: unknown = null;
  private noted = false;
  /** V1051-04 The one message under the preview, and the one Undo (a pick, a combo, a delete). */
  note = $state.raw<{ text: string; undo: (() => void) | null } | null>(null);
  private noteTimer: ReturnType<typeof setTimeout> | undefined;
  readonly sel: Selection;

  constructor(account: string) {
    this.account = account;
    this.sel = new Selection(
      (kind, id) => cosmeticsIpc.select(account, kind, id),
      () => this.changed(),
    );
  }

  status = $derived(this.failed ? 'down' : (this.view?.status ?? 'ready'));
  paintRows = $derived((this.view?.paints ?? []).map(toPaint));
  badgeRows = $derived((this.view?.badges ?? []).map(toBadge));
  paintsShown = $derived(filterItems(this.paintRows, this.paintsQ, this.now));
  badgesShown = $derived(filterItems(this.badgeRows, this.badgesQ, this.now));
  paintById = $derived(new Map((this.view?.paints ?? []).map((p) => [p.style.id, p])));
  badgeById = $derived(new Map((this.view?.badges ?? []).map((b) => [b.def.id, b])));
  anyUsable = $derived(this.paintRows.some((r) => r.usable) || this.badgeRows.some((r) => r.usable));
  allUsable = $derived(this.paintRows.every((r) => r.usable) && this.badgeRows.every((r) => r.usable));
  shown = $derived.by(() => {
    void this.tick;
    return this.sel.shown;
  });
  saveStatus = $derived.by(() => {
    void this.tick;
    return this.sel.status;
  });
  canUndo = $derived.by(() => {
    void this.tick;
    return this.note ? this.note.undo !== null : this.sel.undoable !== null;
  });

  /** Says something; with an Undo, or `fade`, for five seconds (like "Saved"). */
  say(text: string, undo: (() => void) | null = null, fade = undo !== null) {
    clearTimeout(this.noteTimer);
    this.note = { text, undo };
    if (fade) this.noteTimer = setTimeout(() => (this.note = null), UNDO_MS);
  }

  clearNote() {
    clearTimeout(this.noteTimer);
    this.note = null;
  }

  /** Undo and Ctrl+Z: whichever happened last. */
  undo() {
    const n = this.note;
    if (n?.undo) {
      this.clearNote();
      n.undo();
    } else this.sel.undo();
  }

  private changed() {
    this.tick++;
    if (this.sel.status === 'saving') this.clearNote();
    const s = this.sel.status;
    if (typeof s !== 'object' || s === this.handled) return;
    this.handled = s;
    toast.error(errorText(s.error));
    if (s.error.type === 'signInAgain' || s.error.type === 'signedOut' || s.error.type === 'noConsent') void this.load();
  }

  liveKey(): string {
    const a = accounts.list.find((x) => x.userId === this.account);
    return a ? `${a.seventv.consent}${a.seventv.signedIn}` : '';
  }

  async load(fresh = false) {
    this.loading = true;
    this.key = this.liveKey();
    try {
      this.apply(await cosmeticsIpc.view(this.account, fresh));
    } catch {
      this.failed = true;
    }
    this.loading = false;
  }

  private apply(v: CosmeticsView) {
    this.failed = false;
    this.view = v;
    this.sel.reset(v.paint, v.badge);
    this.sheet?.drop();
    this.sheet = null;
    if (v.paints.length) {
      this.sheet = menuSheet(v.paints.map((p) => p.style));
      this.sheet.adopt();
    }
  }

  async allow() {
    // Loading, so the sign-in change that Allow causes doesn't start a second fetch.
    this.loading = true;
    try {
      this.apply(await cosmeticsIpc.allow(this.account));
      this.key = this.liveKey();
    } catch {
      this.failed = true;
    }
    this.loading = false;
  }

  async signIn() {
    try {
      const r = await cosmeticsIpc.signIn(this.account);
      if (r.type === 'done') await this.load();
      else if (r.type === 'cancelled') toast.info('Sign-in cancelled.');
      else if (r.type === 'otherAccount') {
        const name = accounts.list.find((a) => a.userId === this.account)?.displayName ?? 'this account';
        toast.error(`That’s a different Twitch account. Sign in as ${name}.`);
      } else toast.error('Sign-in didn’t finish. Try again.');
    } catch {
      toast.error('Sign-in didn’t finish. Try again.');
    }
  }

  async signOut() {
    try {
      await cosmeticsIpc.signOut(this.account);
    } catch {
      // The reload below shows where things stand.
    }
    await this.load();
  }

  /** Picks one; a locked item only says why, once per open. */
  pick(kind: Kind, id: string | null, usable: boolean) {
    if (!usable) {
      if (!this.noted) toast.info('That one’s for 7TV subscribers.');
      this.noted = true;
      return;
    }
    this.sel.pick(kind, id);
  }

  destroy() {
    clearTimeout(this.noteTimer);
    this.sheet?.drop();
    this.sheet = null;
  }
}
