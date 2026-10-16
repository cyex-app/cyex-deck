// C4 Polls and predictions per chat, kept current by Rust (GQL once, then Hermes).

import { SvelteMap } from 'svelte/reactivity';

import type { VotesUpdate } from '../../ipc/bindings/VotesUpdate';
import { chatxIpc } from '../../ipc/chatx';

export interface ChatVotes extends VotesUpdate {
  /** When this arrived, to count the poll's time down. */
  at: number;
}

class Votes {
  readonly byChannel = new SvelteMap<string, ChatVotes>();
  private listening = false;

  /** Starts listening (once per window) and reads what a chat has now. */
  watch(login: string): void {
    if (!this.listening) {
      this.listening = true;
      void chatxIpc.onVotes((v) => this.byChannel.set(v.channel, { ...v, at: Date.now() }));
    }
    const asked = Date.now();
    chatxIpc
      .votes(login)
      .then((v) => {
        // An event that came in meanwhile is newer than this snapshot.
        const have = v && this.byChannel.get(v.channel);
        if (v && !(have && have.at >= asked)) this.byChannel.set(v.channel, { ...v, at: Date.now() });
      })
      .catch(() => {});
  }
}

export const votes = new Votes();

/** Seconds left, never below 0. */
export function pollLeft(v: ChatVotes, now: number): number {
  if (!v.poll || v.poll.status !== 'ACTIVE') return 0;
  return Math.max(0, Math.round((v.poll.remainingMs - (now - v.at)) / 1000));
}

export function predictionLeft(v: ChatVotes, now: number): number {
  const p = v.prediction;
  if (!p || p.status !== 'ACTIVE') return 0;
  const ends = Date.parse(p.createdAt) + p.windowSecs * 1000;
  return Number.isNaN(ends) ? 0 : Math.max(0, Math.round((ends - now) / 1000));
}
