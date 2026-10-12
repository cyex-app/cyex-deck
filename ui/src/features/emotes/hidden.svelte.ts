// E4 Emotes the user hid: shown as their name everywhere.

import type { EmoteKey } from '../../ipc/bindings/EmoteKey';
import type { Provider } from '../../ipc/bindings/Provider';
import { settings } from '../../lib/settings.svelte';
import { emoteKey } from './emotes';

class Hidden {
  private keys = $derived(new Set(settings.v.chat.hiddenEmotes.map((k) => emoteKey(k.source, k.id))));

  has(provider: Provider, id: string): boolean {
    return this.keys.has(emoteKey(provider, id));
  }

  hide(e: EmoteKey) {
    if (this.has(e.source as Provider, e.id)) return;
    void settings.patch('chat', { hiddenEmotes: [...settings.v.chat.hiddenEmotes, e] });
  }

  show(source: string, id: string) {
    void settings.patch('chat', {
      hiddenEmotes: settings.v.chat.hiddenEmotes.filter((k) => emoteKey(k.source, k.id) !== emoteKey(source as Provider, id)),
    });
  }
}

export const hidden = new Hidden();
