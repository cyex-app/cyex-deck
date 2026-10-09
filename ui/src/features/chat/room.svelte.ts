// One open chat view: the messages it shows and the room's state, kept in sync with
// Rust through batched events (1.8). Rust does the parsing; this only applies results.

import { chatIpc } from '../../ipc/chat';
import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { ConnStatus } from '../../ipc/bindings/ConnStatus';
import type { Pinned } from '../../ipc/bindings/Pinned';
import type { RoomEvent } from '../../ipc/bindings/RoomEvent';
import type { RoomState } from '../../ipc/bindings/RoomState';

/** Lines kept on screen, like Twitch's own chat (about 150). Older ones drop off the top;
 * Rust keeps 500 per room for usercards. */
export const MAX_LINES = 200;

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
  /** The account reading this chat, `null` when anonymous. */
  account = $state<string | null>(null);
  error = $state('');
  view = 0;
  closed = false;
  /** Batches that arrive before the snapshot; applied right after it. */
  private early: RoomEvent[][] | null = [];

  constructor(readonly login: string) {}

  async open(account: string | null): Promise<void> {
    try {
      const opened = await chatIpc.open(this.login, account, (events) =>
        this.early ? this.early.push(events) : this.apply(events),
      );
      if (this.closed) {
        void chatIpc.close(opened.view);
        return;
      }
      this.view = opened.view;
      this.account = opened.account;
      const s = opened.snapshot;
      this.messages = s.messages.slice(-MAX_LINES);
      this.state = s.state;
      this.status = s.status;
      this.pinned = s.pinned;
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
        case 'history': {
          const known = new Set(list.map((m) => m.id));
          list = [...e.messages.filter((m) => !known.has(m.id)), ...list];
          changed = true;
          break;
        }
        case 'pinned':
          this.pinned = e.pin;
          break;
        case 'state':
          this.state = e.state;
          break;
        case 'status':
          this.status = e.status;
          break;
      }
    }
    if (changed) this.messages = list.length > MAX_LINES ? list.slice(-MAX_LINES) : list;
  }
}

function deleted(m: ChatMessage): ChatMessage | null {
  return m.flags.includes('deleted') ? null : { ...m, flags: [...m.flags, 'deleted'] };
}
