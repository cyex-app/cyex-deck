// Follows a Twitch link from the bell or a toast (1.26): channel → tab, other twitch.tv
// pages → the app's Twitch window for the default account, anything else → the browser.

import { subsIpc } from '../../ipc/subs';
import { systemIpc } from '../../ipc/system';
import { accounts } from '../../lib/accounts.svelte';
import { toast } from '../../lib/toast.svelte';
import { route } from './route';

export function follow(url: string, onchannel: (login: string) => void): void {
  const r = route(url);
  if (r.kind === 'channel') return onchannel(r.login);
  const account = accounts.current?.userId;
  const fail = (e: unknown) => toast.error(String(e));
  if (r.kind === 'twitch' && account) subsIpc.open(account, r.url).catch(fail);
  else systemIpc.openChatLink(r.url).catch(fail);
}
