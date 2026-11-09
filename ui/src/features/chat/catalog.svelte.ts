// Badge and cheermote images (1.40). Global ones load once per window; each channel's
// load when its chat opens. Channel entries win over global ones. V108-BD A badge a loaded
// catalog lacks asks Rust to read it again; new ones arrive for every window (`chat:catalog`).

import { SvelteMap } from 'svelte/reactivity';

import { chatIpc } from '../../ipc/chat';
import type { BadgeInfo } from '../../ipc/bindings/BadgeInfo';
import type { CatalogUpdate } from '../../ipc/bindings/CatalogUpdate';
import type { Cheermote } from '../../ipc/bindings/Cheermote';
import { BadgeMisses, RetryGate, missScope } from './badgeMiss';
import type { BadgeRef } from './sharedBadges';

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
  private misses = new BadgeMisses();
  /** V110-F84 Channels asked for: a push for one of them lands even while the answer is out or was empty. */
  private asked = new Set<string>();
  private sourceFails = new RetryGate();

  async loadGlobal(): Promise<void> {
    if (this.loading.has('')) return;
    this.loading.add('');
    // V108-BD Once per window: catalogs read again come in here.
    if (!this.listening) {
      this.listening = true;
      void chatIpc.onCatalog((u) => this.apply(u)).catch(() => (this.listening = false));
    }
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
    this.asked.add(login);
    try {
      const c = await chatIpc.channelCatalog(login);
      if (c) this.channels.set(login, index(c.badges, c.cheermotes));
      // V110-F84 No answer yet: a later open asks again; the push fills it meanwhile.
      else this.loading.delete(login);
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
    if (this.loading.has(key) || !this.sourceFails.ok(key)) return;
    this.loading.add(key);
    try {
      const room = await chatIpc.sourceBadges(roomId);
      this.sources.set(roomId, new Map(room.badges.map((b) => [`${b.set}/${b.version}`, b])));
      if (room.login) this.rooms.set(roomId, { login: room.login, name: room.name, avatar: room.avatar ?? null });
      this.sourceFails.done(key);
    } catch {
      this.sourceFails.fail(key);
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

  private listening = false;

  /** V108-BD A catalog read again; only ones this window holds are replaced. */
  private apply(u: CatalogUpdate) {
    if (u.kind === 'global') this.global = index(u.catalog.badges, u.catalog.cheermotes);
    else if (u.kind === 'channel') {
      if (this.channels.has(u.login) || this.asked.has(u.login)) this.channels.set(u.login, index(u.catalog.badges, u.catalog.cheermotes));
    } else if (this.sources.has(u.id)) {
      this.sources.set(u.id, new Map(u.room.badges.map((b) => [`${b.set}/${b.version}`, b])));
      if (u.room.login) this.rooms.set(u.id, { login: u.room.login, name: u.room.name, avatar: u.room.avatar ?? null });
    }
  }

  /** V108-BD A line's Twitch badge that neither its catalog nor the global one has, once
   *  both have loaded: Rust reads them again (throttled), the first time per badge. */
  missing(channel: string, ref: BadgeRef): void {
    if (!this.global.badges.size) return;
    const own = ref.room ? this.sources.get(ref.room) : this.channels.get(channel)?.badges;
    if (!own) return;
    const key = `${ref.set}/${ref.version}`;
    if (own.has(key) || this.global.badges.has(key)) return;
    if (!this.misses.first(missScope(channel, ref.room), ref.set, ref.version)) return;
    void chatIpc.badgeMissing(ref.room ? null : channel, ref.room ?? null).catch(() => {});
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
