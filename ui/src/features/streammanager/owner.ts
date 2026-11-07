// V109-C04 Whose channel a dashboard panel acts on. In the stream manager: its account, and
// no account goes with the calls (Rust uses the manager's). In the creator dashboard: the
// account picked in its header, sent with every call (`setOwner`).

import { getContext, setContext } from 'svelte';

import type { SmBroadcaster } from '../../ipc/bindings/SmBroadcaster';
import { sm } from './streammanager.svelte';

export interface Owner {
  /** Whose channel; null while not known. */
  readonly who: SmBroadcaster | null;
  /** Sent with each call; null lets Rust use the stream manager's account. */
  readonly account: string | null;
}

const KEY = Symbol('sm-owner');

const MANAGER: Owner = {
  get who() {
    return sm.manager;
  },
  account: null,
};

/** For the panels inside: they act on this owner's channel. */
export function setOwner(owner: Owner): void {
  setContext(KEY, owner);
}

/** In a panel's setup: the dashboard's owner, else the stream manager's. */
export function useOwner(): Owner {
  return getContext<Owner | undefined>(KEY) ?? MANAGER;
}
