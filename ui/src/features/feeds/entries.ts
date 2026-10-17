// V106-07 The list a session feed shows, kept from the events Rust sends. Newest first.

import type { FeedEntry } from '../../ipc/bindings/FeedEntry';
import type { KindEvent } from '../../ipc/bindings/KindEvent';

export function keyOf(e: FeedEntry): string {
  switch (e.type) {
    case 'live':
    case 'held':
    case 'whisper':
      return e.key;
  }
}

export function tsOf(e: FeedEntry): number {
  switch (e.type) {
    case 'live':
    case 'held':
    case 'whisper':
      return e.ts;
  }
}

/** The list after `events`, cut to `cap` from the end. */
export function apply(list: FeedEntry[], events: KindEvent[], cap: number): FeedEntry[] {
  let next = list;
  for (const ev of events) {
    switch (ev.type) {
      case 'add': {
        const at = next.findIndex((e) => keyOf(e) === keyOf(ev.entry));
        next = at >= 0 ? next.map((e, i) => (i === at ? ev.entry : e)) : [ev.entry, ...next];
        break;
      }
      case 'update': {
        const key = keyOf(ev.entry);
        if (next.some((e) => keyOf(e) === key)) next = next.map((e) => (keyOf(e) === key ? ev.entry : e));
        break;
      }
      case 'remove':
        next = next.filter((e) => keyOf(e) !== ev.key);
        break;
      case 'clear':
        next = [];
        break;
    }
  }
  return next.length > cap ? next.slice(0, cap) : next;
}
