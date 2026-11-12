// V110-N13 What the poll and prediction cards call: the stream manager's own channel
// (`account` null, as before), or a channel the account moderates (the mod view, `/poll`,
// `/prediction`), where the card also keeps the chat's vote watcher on while it shows.

import type { SmPoll } from '../../ipc/bindings/SmPoll';
import type { SmPrediction } from '../../ipc/bindings/SmPrediction';
import type { SmPredictionEnd } from '../../ipc/bindings/SmPredictionEnd';
import { chatIpc } from '../../ipc/chat';
import { modVotesIpc, smIpc } from '../../ipc/streammanager';
import { votes } from '../chat/votes.svelte';
import { votesAllowed } from './voteGate';

export interface VoteCalls {
  createPoll(poll: SmPoll): Promise<void>;
  endPoll(id: string, archive: boolean): Promise<void>;
  createPrediction(prediction: SmPrediction): Promise<void>;
  endPrediction(id: string, end: SmPredictionEnd): Promise<void>;
  /** Follows the channel's votes while the card shows; returns the cleanup. */
  watch(): () => void;
}

export function voteCalls(login: string, account: string | null): VoteCalls {
  if (!account) {
    return {
      createPoll: smIpc.createPoll,
      endPoll: smIpc.endPoll,
      createPrediction: smIpc.createPrediction,
      endPrediction: smIpc.endPrediction,
      // The stream manager keeps its own channel's watcher on.
      watch: () => {
        votes.watch(login);
        return () => {};
      },
    };
  }
  return {
    createPoll: (p) => modVotesIpc.createPoll(account, login, p),
    endPoll: (id, archive) => modVotesIpc.endPoll(account, login, id, archive),
    createPrediction: (p) => modVotesIpc.createPrediction(account, login, p),
    endPrediction: (id, end) => modVotesIpc.endPrediction(account, login, id, end),
    watch: () => {
      modVotesIpc
        .watch(account, login)
        .then(() => votes.watch(login))
        .catch(() => {});
      return () => void modVotesIpc.unwatch(login).catch(() => {});
    },
  };
}

/** One profile read per channel and session; a failed read is tried again next time. */
const allowedCache = new Map<string, Promise<boolean | null>>();

export function channelVotesAllowed(login: string): Promise<boolean | null> {
  let p = allowedCache.get(login);
  if (!p) {
    p = chatIpc.profile(login).then(votesAllowed);
    p.catch(() => allowedCache.delete(login));
    allowedCache.set(login, p);
  }
  return p.catch(() => null);
}
