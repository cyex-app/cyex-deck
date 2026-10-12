// Badge and cheermote images (1.40). Global ones load once per window; each channel's
// load when its chat opens. Channel entries win over global ones.

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
  channels = $state.raw<Record<string, Catalog>>({});
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
      if (c) this.channels = { ...this.channels, [login]: index(c.badges, c.cheermotes) };
    } catch {
      this.loading.delete(login);
    }
  }

  badge(channel: string, set: string, version: string): BadgeInfo | undefined {
    const key = `${set}/${version}`;
    return this.channels[channel]?.badges.get(key) ?? this.global.badges.get(key);
  }

  cheermote(channel: string, prefix: string): Cheermote | undefined {
    const key = prefix.toLowerCase();
    return this.channels[channel]?.cheermotes.get(key) ?? this.global.cheermotes.get(key);
  }
}

export const catalogs = new Catalogs();
