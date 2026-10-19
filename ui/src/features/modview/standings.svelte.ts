// V107-M8 Mod and VIP changes seen in this window (`mod:standing`), per channel and user.

import { modIpc } from '../../ipc/moderation';
import { applyChange, merged, type Standing } from './standing';

/** Users remembered at once; the oldest go first. */
const CAP = 500;

const changes = $state<Record<string, Partial<Standing>>>({});
const key = (channel: string, login: string) => `${channel}/${login}`.toLowerCase();

let listening = false;
function listen() {
  if (listening) return;
  listening = true;
  void modIpc.onStanding((c) => {
    const k = key(c.channel, c.login);
    const prev = changes[k];
    delete changes[k];
    changes[k] = applyChange(prev, c);
    const keys = Object.keys(changes);
    if (keys.length > CAP) delete changes[keys[0]];
  });
}

export const standings = {
  /** What `login` holds in `channel`: `seen` (badges, a lookup) with changes since on top. */
  of(channel: string, login: string, seen?: Standing): Standing | undefined {
    listen();
    return merged(seen, changes[key(channel, login)]);
  },
};
