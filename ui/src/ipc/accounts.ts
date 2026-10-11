import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { AuthSnapshot } from './bindings/AuthSnapshot';
import type { ConnectionCount } from './bindings/ConnectionCount';
import type { DeviceStart } from './bindings/DeviceStart';
import type { ManualToken } from './bindings/ManualToken';
import type { TokenKind } from './bindings/TokenKind';

export const accountsIpc = {
  snapshot: () => invoke<AuthSnapshot>('accounts_snapshot'),
  /** Device Code sign-in. `onCode` gets the code to show; resolves with the display name. */
  signIn: (kind: TokenKind, onCode: (code: DeviceStart) => void) => {
    const channel = new Channel<DeviceStart>();
    channel.onmessage = onCode;
    return invoke<string>('accounts_sign_in', { kind, onCode: channel });
  },
  cancelSignIn: () => invoke<void>('accounts_cancel_sign_in'),
  /** V101-08 A token someone has, with Chatterino's other fields; resolves with the display name. */
  addManual: (fields: ManualToken) => invoke<string>('accounts_add_manual', { fields }),
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
