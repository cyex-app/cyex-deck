// 1.13 The joined-chats list: per account, each channel it has open, and whether a player
// for that channel is open too.

import type { ConnectionCount } from '../../ipc/bindings/ConnectionCount';

export interface JoinedChannel {
  login: string;
  /** A stream is open for it, not just the chat. */
  stream: boolean;
}

export interface JoinedGroup {
  userId: string;
  name: string;
  channels: JoinedChannel[];
}

/** `streams`: logins with an open player. The current account first, then by name;
 * channels with a stream first, then by login. Accounts with nothing joined are left out. */
export function joinedGroups(
  counts: ConnectionCount[],
  current: string,
  name: (id: string) => string,
  streams: Iterable<string>,
): JoinedGroup[] {
  const live = new Set([...streams].map((l) => l.toLowerCase()));
  return counts
    .filter((c) => c.channels.length)
    .map((c) => ({
      userId: c.userId,
      name: name(c.userId),
      channels: c.channels
        .map((login) => ({ login, stream: live.has(login.toLowerCase()) }))
        .sort((a, b) => Number(b.stream) - Number(a.stream) || a.login.localeCompare(b.login)),
    }))
    .sort((a, b) => Number(b.userId === current) - Number(a.userId === current) || a.name.localeCompare(b.name));
}
