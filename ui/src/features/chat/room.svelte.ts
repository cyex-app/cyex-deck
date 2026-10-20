// One open chat view: the messages it shows and the room's state, kept in sync with
// Rust through batched events (1.8). Rust does the parsing; this only applies results.

import { chatIpc } from '../../ipc/chat';
import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { ConnStatus } from '../../ipc/bindings/ConnStatus';
import type { Pinned } from '../../ipc/bindings/Pinned';
import type { RoomEvent } from '../../ipc/bindings/RoomEvent';
import type { RoomState } from '../../ipc/bindings/RoomState';
import { settings } from '../../lib/settings.svelte';
import { CappedMap } from '../../lib/capped';
import { errorText } from '../../lib/errors';
import { nextPiece, prepend } from './historyDrip';

/** V101-02 Lines kept on screen, as set; older ones drop off the top. Rust keeps as many
 * per shown room, at least 500. V107-M1 A room shown again after the background opens with
 * the 150 it kept; its older lines follow in one `history` event and go on
 * screen 25 per frame (historyDrip.ts). */
const maxLines = () => settings.v.chat?.scrollback ?? 1000;

/** C5 Last message seen per tile, for the last-read line after switching tabs. L1 Capped:
 *  closed tiles never come back. */
export const readMarks = new CappedMap<string, string>(300);

const EMPTY_STATE: RoomState = {
  channelId: null,
  slow: 0,
  followersOnly: null,
  subsOnly: false,
  emoteOnly: false,
  unique: false,
  own: null,
  waitUntil: null,
};

export class ChatRoom {
  /** Replaced, never mutated, so the list re-renders cheaply. */
  messages = $state.raw<ChatMessage[]>([]);
  state = $state.raw<RoomState>(EMPTY_STATE);
  status = $state<ConnStatus>('connecting');
  pinned = $state.raw<Pinned | null>(null);
  /** V103-04 The pin was read: a null `pinned` means nothing is pinned, not "not yet". */
  pinKnown = $state(false);
  /** The account reading this chat, `null` when anonymous. */
  account = $state<string | null>(null);
  error = $state('');
  /** Set once Rust opened the view; effects can wait on it (V101-13). */
  view = $state(0);
  closed = false;
  /** Batches that arrive before the snapshot; applied right after it. */
  private early: RoomEvent[][] | null = [];
  /** V107-M1 History not on screen yet, drawn a piece per frame. */
  private later: ChatMessage[][] = [];
  private frame = 0;

  constructor(readonly login: string) {}

  /** `filters`: F3 filter ids. `follows`: a view's own chat (V103-22). */
  async open(account: string | null, filters: string[] = [], follows = false): Promise<void> {
    try {
      const opened = await chatIpc.open(
        this.login,
        account,
        filters,
        (events) => (this.early ? this.early.push(events) : this.apply(events)),
        follows,
      );
      if (this.closed) {
        void chatIpc.close(opened.view);
        return;
      }
      this.view = opened.view;
      this.account = opened.account;
      const s = opened.snapshot;
      this.messages = s.messages.slice(-maxLines());
      this.state = s.state;
      this.status = s.status;
      this.pinned = s.pinned;
      this.pinKnown = s.pinKnown;
      const early = this.early ?? [];
      this.early = null;
      for (const events of early) this.apply(events);
    } catch (e) {
      this.error = errorText(e);
    }
  }

  close(): void {
    this.closed = true;
    if (this.frame) cancelAnimationFrame(this.frame);
    this.later = [];
    if (this.view) void chatIpc.close(this.view);
  }

  find(id: string): ChatMessage | undefined {
    return this.messages.find((m) => m.id === id);
  }

  apply(events: RoomEvent[]): void {
    let list = this.messages;
    let changed = false;
    let history = false;
    /** Ids on screen, made once a batch has a message: a repeated id anywhere would break the keyed list. */
    let ids: Set<string> | null = null;
    const edit = (fn: (m: ChatMessage) => ChatMessage | null) => {
      list = list.map((m) => fn(m) ?? m);
      this.later = this.later.map((c) => c.map((m) => fn(m) ?? m));
      changed = true;
    };
    for (const e of events) {
      switch (e.type) {
        case 'message':
          // A repeated id would break the keyed list and stop the chat (T3): checked against every line.
          ids ??= new Set(list.map((m) => m.id));
          if (ids.has(e.message.id)) break;
          ids.add(e.message.id);
          list = changed ? list : list.slice();
          list.push(e.message);
          changed = true;
          break;
        case 'update':
          edit((m) => (m.id === e.message.id ? e.message : null));
          break;
        case 'delete':
          edit((m) => (m.id === e.id ? deleted(m) : null));
          break;
        case 'clearUser':
          edit((m) => (!m.notice && m.sender?.id === e.userId ? deleted(m) : null));
          break;
        // S14 A plugin took its line back, or emptied the chat on screen.
        case 'remove':
          list = list.filter((m) => m.id !== e.id);
          ids?.delete(e.id);
          this.later = this.later.map((c) => c.filter((m) => m.id !== e.id));
          changed = true;
          break;
        case 'wipe':
          list = [];
          ids = null;
          this.later = [];
          changed = true;
          break;
        case 'history':
          this.later.push(e.messages.slice());
          history = true;
          break;
        case 'gap':
          list = mergeByTime(list, e.messages);
          ids = null;
          changed = true;
          break;
        case 'pinned':
          this.pinned = e.pin;
          this.pinKnown = true;
          break;
        case 'state':
          this.state = e.state;
          break;
        case 'status':
          this.status = e.status;
          break;
      }
    }
    if (changed) this.set(list);
    // After the set: a hidden window drips at once, and must not be overwritten by `list`.
    if (history) this.drip();
  }

  private set(list: ChatMessage[]): void {
    const max = maxLines();
    this.messages = list.length > max ? list.slice(-max) : list;
  }

  /** V107-M1 One piece of queued history per frame, so a refill never draws 100 lines at once. */
  private drip(): void {
    if (this.closed) return;
    // L2 Frames don't run in a hidden window: the queue would wait there. Nothing is drawn, so all of it goes in at once.
    if (typeof document !== 'undefined' && document.hidden) {
      if (this.frame) cancelAnimationFrame(this.frame);
      this.frame = 0;
      while (this.later.length && this.messages.length < maxLines()) {
        const piece = nextPiece(this.later);
        if (piece.length) this.set(prepend(this.messages, piece));
      }
      this.later = [];
      return;
    }
    if (this.frame) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      // Full already: older lines would only drop off again.
      if (this.messages.length >= maxLines()) this.later = [];
      const piece = nextPiece(this.later);
      if (piece.length) this.set(prepend(this.messages, piece));
      if (this.later.length) this.drip();
    });
  }
}

/** C24 Missed messages go before the first chat line that's newer. */
function mergeByTime(list: ChatMessage[], missed: ChatMessage[]): ChatMessage[] {
  const known = new Set(list.map((m) => m.id));
  const out = list.slice();
  for (const m of missed) {
    if (known.has(m.id)) continue;
    const at = out.findIndex((x) => x.sender && x.ts > m.ts);
    out.splice(at === -1 ? out.length : at, 0, m);
  }
  return out;
}

function deleted(m: ChatMessage): ChatMessage | null {
  return m.flags.includes('deleted') ? null : { ...m, flags: [...m.flags, 'deleted'] };
}
