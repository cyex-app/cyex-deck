// One open chat view: the messages it shows and the room's state, kept in sync with
// Rust through batched events (1.8). Rust does the parsing; this only applies results.

import { chatIpc } from '../../ipc/chat';
import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { ConnStatus } from '../../ipc/bindings/ConnStatus';
import type { Pinned } from '../../ipc/bindings/Pinned';
import type { RoomEvent } from '../../ipc/bindings/RoomEvent';
import type { RoomState } from '../../ipc/bindings/RoomState';
import { settings } from '../../lib/settings.svelte';

/** V101-02 Lines kept on screen, as set; older ones drop off the top. Rust keeps as many
 * per room, and at least 500 for usercards. */
const maxLines = () => settings.v.chat?.scrollback ?? 1000;

/** C5 Last message seen per tile, for the last-read line after switching tabs. */
export const readMarks = new Map<string, string>();

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

  constructor(readonly login: string) {}

  /** `filters`: F3 filter ids. */
  async open(account: string | null, filters: string[] = []): Promise<void> {
    try {
      const opened = await chatIpc.open(this.login, account, filters, (events) =>
        this.early ? this.early.push(events) : this.apply(events),
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
      this.error = String(e);
    }
  }

  close(): void {
    this.closed = true;
    if (this.view) void chatIpc.close(this.view);
  }

  find(id: string): ChatMessage | undefined {
    return this.messages.find((m) => m.id === id);
  }

  apply(events: RoomEvent[]): void {
    let list = this.messages;
    let changed = false;
    const edit = (fn: (m: ChatMessage) => ChatMessage | null) => {
      list = list.map((m) => fn(m) ?? m);
      changed = true;
    };
    for (const e of events) {
      switch (e.type) {
        case 'message':
          // A repeated id would break the keyed list.
          if (list.slice(-50).some((m) => m.id === e.message.id)) break;
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
          changed = true;
          break;
        case 'wipe':
          list = [];
          changed = true;
          break;
        case 'history': {
          const known = new Set(list.map((m) => m.id));
          list = [...e.messages.filter((m) => !known.has(m.id)), ...list];
          changed = true;
          break;
        }
        case 'gap':
          list = mergeByTime(list, e.messages);
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
    const max = maxLines();
    if (changed) this.messages = list.length > max ? list.slice(-max) : list;
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
