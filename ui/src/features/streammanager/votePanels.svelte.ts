// V110-N13 The floating poll and prediction panels `/poll` and `/prediction` open: one of
// each kind per window. The same kind again focuses the open one (or, for another channel,
// replaces it). Mounted on the page body, so it floats over whatever the window shows.

import { mount, unmount } from 'svelte';

import VoteFloat from './VoteFloat.svelte';
import type { VoteKind } from './voteGate';

interface Open {
  login: string;
  panel: { focus: () => void };
  /** Where focus was (the chat box): it goes back there on close. */
  back: HTMLElement | null;
}

const open = new Map<VoteKind, Open>();

export function closeVote(kind: VoteKind): void {
  const o = open.get(kind);
  if (!o) return;
  open.delete(kind);
  void unmount(o.panel);
  if (o.back?.isConnected) o.back.focus();
}

export function openVote(kind: VoteKind, login: string, account: string, question: string): void {
  const have = open.get(kind);
  if (have?.login === login) {
    have.panel.focus();
    return;
  }
  closeVote(kind);
  const back = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const panel = mount(VoteFloat, {
    target: document.body,
    props: { kind, login, account, question, nth: open.size, onclose: () => closeVote(kind) },
  });
  open.set(kind, { login, panel, back });
}
