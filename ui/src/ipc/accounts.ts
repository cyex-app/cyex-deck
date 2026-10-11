import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { AuthSnapshot } from './bindings/AuthSnapshot';
import type { ConnectionCount } from './bindings/ConnectionCount';
import type { DeviceStart } from './bindings/DeviceStart';
import type { SignedIn } from './bindings/SignedIn';
import type { TokenKind } from './bindings/TokenKind';

export const accountsIpc = {
  snapshot: () => invoke<AuthSnapshot>('accounts_snapshot'),
  /** Device Code sign-in. `onCode` gets the code to show; resolves with the account. With
   * `userId`, the sign-in is added to that account and must be the same user (V102-19). */
  signIn: (kind: TokenKind, userId: string | null, onCode: (code: DeviceStart) => void) => {
    const channel = new Channel<DeviceStart>();
    channel.onmessage = onCode;
    return invoke<SignedIn>('accounts_sign_in', { kind, userId, onCode: channel });
  },
  cancelSignIn: () => invoke<void>('accounts_cancel_sign_in'),
  /** V102-19 A GQL token for an account signed in with Twitch. */
  addGqlToken: (userId: string, token: string) => invoke<SignedIn>('accounts_add_gql_token', { userId, token }),
  /** V102-20 Brings the main window up on the sign-in that unlocks a greyed-out tool. */
  showSignIn: (kind: TokenKind) => invoke<void>('accounts_show_sign_in', { kind }),
  onShowSignIn: (handler: (kind: TokenKind) => void) =>
    listen<TokenKind>('accounts:sign-in', (e) => handler(e.payload)),
  remove: (userId: string) => invoke<void>('accounts_remove', { userId }),
  removeToken: (userId: string, kind: TokenKind) => invoke<void>('accounts_remove_token', { userId, kind }),
  setDefault: (userId: string) => invoke<void>('accounts_set_default', { userId }),
  setClientId: (clientId: string) => invoke<void>('accounts_set_client_id', { clientId }),
  connections: () => invoke<ConnectionCount[]>('connections_counts'),
  onChanged: (handler: (snapshot: AuthSnapshot) => void) =>
    listen<AuthSnapshot>('accounts:changed', (e) => handler(e.payload)),
  onConnections: (handler: (counts: ConnectionCount[]) => void) =>
    listen<ConnectionCount[]>('connections:changed', (e) => handler(e.payload)),
};
