// V109-B16 What the cyex pop-up says when the account can't get a cyex session: no Twitch
// sign-in on it (only Extra Twitch features), or the server refused that sign-in.

import type { CyexView } from '../../ipc/cyex';

export type SignInNeed = { text: string; button: string };

export function signInNeed(v: CyexView): SignInNeed | null {
  if (v.state === 'noToken') return { text: 'cyex needs the Twitch sign-in on this account.', button: 'Sign in' };
  if (v.state === 'refused') return { text: 'cyex didn’t take this sign-in. Sign in again.', button: 'Sign in' };
  return null;
}
