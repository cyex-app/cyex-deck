// 1.44 "Mine first" from a tab's menu: up to two channels the miner takes before all
// others. A third pushes the older one out.

import type { MenuEntry } from '../../components/menu.svelte';
import type { TabChannel } from '../../ipc/bindings/TabChannel';
import { minerIpc } from '../../ipc/miner';
import { accounts } from '../../lib/accounts.svelte';
import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';

async function set(c: TabChannel, on: boolean): Promise<void> {
  const out = await minerIpc.first({ id: c.id, login: c.login, name: c.name }, on);
  if (out) toast.info(`${out} isn’t mined first anymore.`);
  else if (on) toast.info(`${c.name} is mined first.`);
}

/** The menu entry for a tab's channels, or null without an account that can mine. */
export function mineFirstEntry(channels: TabChannel[]): MenuEntry | null {
  if (!channels.length || !accounts.list.some((a) => a.features.includes('miner'))) return null;
  const first = settings.v.miner.first.map((c) => c.id);
  if (channels.length === 1) {
    const c = channels[0];
    const on = first.includes(c.id);
    return { label: 'Mine first', icon: 'pickaxe', checked: on, run: () => void set(c, !on) };
  }
  return {
    label: 'Mine first',
    icon: 'pickaxe',
    items: channels.map((c) => {
      const on = first.includes(c.id);
      return { label: c.name, checked: on, run: () => void set(c, !on) };
    }),
  };
}
