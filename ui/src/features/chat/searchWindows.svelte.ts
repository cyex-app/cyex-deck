// V111-33 The chat search windows: one per chat (view). Ctrl+F in a chat that has one open
// focuses it. Mounted on the page body, so it floats over whatever the window shows.

import { mount, unmount } from 'svelte';

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import SearchFloat from './SearchFloat.svelte';

interface Open {
  /** V111-33 Reactive props: `away` hides the window while its chat's tab is not shown. */
  props: { away: boolean };
  win: { focus: () => void; set: (q: string, all?: boolean) => void };
  /** Where focus was (the chat box): it goes back there on close when nothing else has it. */
  back: HTMLElement | null;
}

const open = new Map<number, Open>();

export function closeSearch(view: number): void {
  const o = open.get(view);
  if (!o) return;
  open.delete(view);
  void unmount(o.win);
  const at = document.activeElement;
  if ((!at || at === document.body) && o.back?.isConnected) o.back.focus();
}

/** V111-33 The window of a chat in a tab that is not shown hides with it, and comes back with it. */
export function searchAway(view: number, away: boolean): void {
  const o = open.get(view);
  if (o) o.props.away = away;
}

export interface SearchOpen {
  view: number;
  channel: string;
  account: string | null;
  query?: string;
  all?: boolean;
  lines: () => ChatMessage[];
}

export function openSearch(o: SearchOpen): void {
  const have = open.get(o.view);
  if (have) {
    have.win.set(o.query ?? '', o.all);
    return;
  }
  const back = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const props = $state({ ...o, nth: open.size, away: false, onclose: () => closeSearch(o.view) });
  const win = mount(SearchFloat, { target: document.body, props });
  open.set(o.view, { props, win, back });
}
