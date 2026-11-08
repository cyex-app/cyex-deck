// V110-U43 The raided channel in a pop-out when a ticked raid goes. Main window only (the
// chat's Rust side tells it). The channel is not added as a tab for good: if you had it as a
// tab of its own that one pops out (closing the window docks it), else a tab is made just for
// the pop-out (`raidTabs`), and it goes again when the window closes without "Dock back".

import { untrack } from 'svelte';

import { popOut } from '../../app/panels';
import type { RaidGo } from '../../ipc/bindings/RaidGo';
import { raidIpc } from '../../ipc/raid';
import { raidTabsToClose } from '../chat/raidBanner';
import { raidTabs } from './raidTabs';
import { tabRoute, tabs, tiles } from './tabs.svelte';

export async function openRaided({ raid: r, account }: RaidGo): Promise<void> {
  if (!r.targetId) return;
  const mine = tabs.list.find((t) => {
    const list = tiles(t);
    return list.length === 1 && list[0].channel.id === r.targetId;
  });
  if (mine) {
    await popOut(tabRoute(mine.id), r.targetName);
    return;
  }
  // Adding makes the new tab active; the main window goes back to what it showed.
  const before = tabs.active;
  const id = await tabs.apply({
    op: 'add',
    channel: { id: r.targetId, login: r.targetLogin, name: r.targetName },
    show: { player: true, chat: true },
    account,
    alias: null,
    into: null,
  });
  if (!id) return;
  raidTabs.add(id);
  tabs.activate(before);
  if (!(await popOut(tabRoute(id), r.targetName))) {
    raidTabs.drop(id);
    tabs.close([id]);
  }
}

/** Call once from the main window's setup. */
export function useRaidPop(): void {
  $effect(() => {
    const off = raidIpc.onGo((go) => void openRaided(go));
    return () => void off.then((f) => f());
  });
  // V110-F18 Marks left by a crash or a closed app: once the windows are known, a mark only
  // stays for a tab that is still there and popped out. The rest lose the mark; no tab closes.
  let swept = false;
  $effect(() => {
    if (!tabs.ready || swept) return;
    swept = true;
    const popped = tabs.popped;
    untrack(() => raidTabs.keep(tabs.list.map((t) => t.id).filter((id) => popped.has(id))));
  });
  // A raid window that closed without docking takes its tab along.
  const seen = new Set<string>();
  $effect(() => {
    const popped = tabs.popped;
    untrack(() => {
      const gone = raidTabsToClose(raidTabs.list(), popped, seen);
      for (const id of gone) raidTabs.drop(id);
      if (gone.length) tabs.close(gone);
    });
  });
}
