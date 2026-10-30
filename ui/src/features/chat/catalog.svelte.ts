// Badge and cheermote images (1.40). Global ones load once per window; each channel's
// load when its chat opens. Channel entries win over global ones.

import { SvelteMap } from 'svelte/reactivity';

import { chatIpc } from '../../ipc/chat';
import type { BadgeInfo } from '../../ipc/bindings/BadgeInfo';
import type { Cheermote } from '../../ipc/bindings/Cheermote';

interface Catalog {
  badges: Map<string, BadgeInfo>;
  cheermotes: Map<string, Cheermote>;
}

const empty = (): Catalog => ({ badges: new Map(), cheermotes: new Map() });

function index(badges: BadgeInfo[], cheermotes: Cheermote[]): Catalog {
  return {
    badges: new Map(badges.map((b) => [`${b.set}/${b.version}`, b])),
    cheermotes: new Map(cheermotes.map((c) => [c.prefix.toLowerCase(), c])),
  };
}

class Catalogs {
  global = $state.raw<Catalog>(empty());
  /** Keyed, so a channel loading wakes only the badges waiting on it, not every line of every chat. */
  channels = new SvelteMap<string, Catalog>();
  private loading = new Set<string>();

  async loadGlobal(): Promise<void> {
    if (this.loading.has('')) return;
    this.loading.add('');
    try {
      const g = await chatIpc.globalCatalog();
      this.global = index(g.badges, g.cheermotes);
    } catch {
      this.loading.delete('');
    }
  }

  async loadChannel(login: string): Promise<void> {
    if (this.loading.has(login)) return;
    this.loading.add(login);
    try {
      const c = await chatIpc.channelCatalog(login);
      if (c) this.channels.set(login, index(c.badges, c.cheermotes));
    } catch {
      this.loading.delete(login);
    }
  }

  /** V108-C10 Shared Chat source rooms' badge sets by room id. */
  sources = new SvelteMap<string, Map<string, BadgeInfo>>();
  /** V108-N12 Who a Shared Chat source room is, for the picture in front of its lines. */
  rooms = new SvelteMap<string, { login: string; name: string; avatar: string | null }>();

  async loadSource(roomId: string): Promise<void> {
    const key = `room:${roomId}`;
    if (this.loading.has(key)) return;
    this.loading.add(key);
    try {
      const room = await chatIpc.sourceBadges(roomId);
      this.sources.set(roomId, new Map(room.badges.map((b) => [`${b.set}/${b.version}`, b])));
      if (room.login) this.rooms.set(roomId, { login: room.login, name: room.name, avatar: room.avatar ?? null });
    } catch {
      this.loading.delete(key);
    }
  }

  /** V108-N12 A source room's name and picture; the first ask reads it (once per room). */
  room(roomId: string): { login: string; name: string; avatar: string | null } | undefined {
    const known = this.rooms.get(roomId);
    // Read after the current update: a derived may ask, and it must not write state itself.
    if (!known) queueMicrotask(() => void this.loadSource(roomId));
    return known;
  }

  /** The source room's picture first, then the global one. Nothing until the room has loaded. */
  sourceBadge(roomId: string, set: string, version: string): BadgeInfo | undefined {
    const key = `${set}/${version}`;
    return this.sources.get(roomId)?.get(key) ?? this.global.badges.get(key);
  }

  badge(channel: string, set: string, version: string): BadgeInfo | undefined {
    const key = `${set}/${version}`;
    return this.channels.get(channel)?.badges.get(key) ?? this.global.badges.get(key);
  }

  cheermote(channel: string, prefix: string): Cheermote | undefined {
    const key = prefix.toLowerCase();
    return this.channels.get(channel)?.cheermotes.get(key) ?? this.global.cheermotes.get(key);
  }

  /** V107-B The channel's cheermotes first, then global ones; each prefix once. */
  cheermotes(channel: string): Cheermote[] {
    const own = [...(this.channels.get(channel)?.cheermotes.values() ?? [])];
    const seen = new Set(own.map((c) => c.prefix.toLowerCase()));
    return [...own, ...[...this.global.cheermotes.values()].filter((c) => !seen.has(c.prefix.toLowerCase()))];
  }
}

export const catalogs = new Catalogs();
