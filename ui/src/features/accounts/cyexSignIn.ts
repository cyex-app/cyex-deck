// V109-B16 What the cyex pop-up says when the account can't get a cyex session: no Sign in with
// Twitch on it (only the code sign-in), or the server refused that sign-in.

import type { CyexView } from '../../ipc/cyex';

export type SignInNeed = { text: string; button: string };

export function signInNeed(v: CyexView): SignInNeed | null {
  if (v.state === 'noToken') return { text: 'cyex needs Sign in with Twitch on this account.', button: 'Sign in with Twitch' };
  if (v.state === 'refused') return { text: 'cyex didn’t take this sign-in. Sign in with Twitch again.', button: 'Sign in with Twitch' };
  return null;
}
