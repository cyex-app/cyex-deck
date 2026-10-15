// V1051-02 The sign-in window call. Group 4 adds `cosmeticsIpc.signIn` and a `SignIn`
// binding; until the two are reconciled this wrapper stands in (one line to swap).

import { invoke } from '@tauri-apps/api/core';

export type SignInResult = { type: 'done' | 'cancelled' | 'otherAccount' | 'failed' };

export const signInCall = (account: string) => invoke<SignInResult>('cosmetics_sign_in', { account });
