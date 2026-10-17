// V106-31/32 Ask Rust what can be done with an emote; wait a moment if it is still loading.

import type { EditOptions } from '../../../ipc/bindings/EditOptions';
import { emotesIpc } from '../../../ipc/emotes';

export async function editOptionsFor(
  account: string | null,
  login: string,
  id: string,
  name: string,
  waitMs = 300,
): Promise<EditOptions | null> {
  if (!account) return null;
  try {
    const first = await emotesIpc.editOptions(account, login, id, name);
    if (first.state !== 'loading') return first;
    await Promise.race([emotesIpc.editWarm(account), new Promise((r) => setTimeout(r, waitMs))]);
    return await emotesIpc.editOptions(account, login, id, name);
  } catch {
    return null;
  }
}
