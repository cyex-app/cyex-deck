// V108-N26 Discord Rich Presence from the main window: the channel in the tab on screen.

import { untrack } from 'svelte';

import { discordIpc } from '../../ipc/discord';
import type { DiscordStatus } from '../../ipc/bindings/DiscordStatus';
import type { Tab } from '../../ipc/bindings/Tab';
import { settings } from '../../lib/settings.svelte';
import { heardTile } from '../tabs/audioFocus';
import { audioFocus } from '../tabs/audioFocus.svelte';
import { streamIds, tabs, tiles } from '../tabs/tabs.svelte';
import { type ChannelLook, watchedTile, watchOf } from './watched';

class Build {
  /** Null until asked. */
  ok = $state<boolean | null>(null);

  async load(): Promise<void> {
    if (this.ok !== null) return;
    try {
      this.ok = await discordIpc.available();
    } catch {
      this.ok = false;
    }
  }
}

/** How the Discord link stands, polled while Settings shows the row (the link changes on its own). */
export function pollDiscordStatus(active: () => boolean, set: (s: DiscordStatus) => void): () => void {
  if (!active()) {
    set('off');
    return () => {};
  }
  const tick = () => void discordIpc.status().then(set, () => set('off'));
  tick();
  const id = setInterval(tick, 3000);
  return () => clearInterval(id);
}

/** Whether this build has a Discord app ID (Settings greys the toggle out without). */
export const discordBuild = new Build();

function lookOf(tab: Tab): ChannelLook | null {
  const all = tiles(tab);
  const players = streamIds(tab).flatMap((id) => {
    const t = all.find((x) => x.id === id);
    return t ? [{ id, channelId: t.channel.id }] : [];
  });
  const live = tabs.liveStreamIds(tab);
  const heard = heardTile(audioFocus.input(tab.id, streamIds(tab), live));
  const tile = watchedTile(players, live, heard);
  const t = tile && all.find((x) => x.id === tile.id);
  if (!t) return null;
  const info = tabs.live[t.channel.id];
  return { login: info?.login ?? t.channel.login, name: info?.name ?? t.channel.name, avatar: info?.avatar ?? null };
}

/** Sends what plays in the tab on screen while Settings → General → Show on Discord is on. */
export function useDiscordPresence(shown: () => Tab | undefined): void {
  $effect(() => {
    void discordBuild.load();
  });
  const key = $derived.by(() => {
    if (!discordBuild.ok || !settings.v.general.discordPresence) return '';
    const tab = shown();
    const look = tab ? lookOf(tab) : null;
    return look ? JSON.stringify(watchOf(look)) : '';
  });
  $effect(() => {
    const k = key;
    untrack(() => void discordIpc.watch(k ? JSON.parse(k) : null).catch(() => {}));
  });
  // The setting on makes Rust check the link now, so Settings can say whether Discord answers.
  const on = $derived(discordBuild.ok === true && settings.v.general.discordPresence);
  $effect(() => {
    const v = on;
    untrack(() => void discordIpc.probe(v).catch(() => {}));
  });
  // The window closing clears it too (Discord drops the activity with the pipe).
  $effect(() => () => void discordIpc.watch(null).catch(() => {}));
}
