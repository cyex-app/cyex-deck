// V110-N4 The rows of the account menu after the account list: grouped by where they go.
// Pure, so a test can read the order.

import type { MenuEntry } from '../../components/menuItems';
import type { TokenKind } from '../../ipc/bindings/TokenKind';

export interface AccountRows {
  /** The Twitch-page rows are greyed out until Sign in with Twitch; the flag says which. */
  walletLock: TokenKind | null;
  /** Twitch's own pages need their own login once: show Connect. */
  connect: (() => void) | null;
  ask(): void;
  onmanage(): void;
  onsubs(): void;
  bits(): void;
  wallet(): void;
  ondashboard(): void;
  oncosmetics(): void;
  oncyex(): void;
}

/** Manage accounts, the creator dashboard, a Twitch submenu, then the cosmetics. */
export function accountRows(a: AccountRows): MenuEntry[] {
  // Locked: the Twitch row itself carries the lock and the unlock flow instead of three greyed rows.
  const twitch: MenuEntry = a.walletLock
    ? { label: 'Twitch', icon: 'twitch', lock: a.walletLock, unlock: a.ask, run: a.ask }
    : {
        label: 'Twitch',
        icon: 'twitch',
        items: [
          { label: 'Subscriptions', icon: 'gift', run: a.onsubs },
          { label: 'Get Bits', icon: 'bits', run: a.bits },
          { label: 'Payment methods', icon: 'card', run: a.wallet },
          // V108-04 These pages keep their own Twitch login.
          ...(a.connect
            ? [{ label: 'Connect Twitch pages', icon: 'login', note: 'Log in once for the wallet, subscriptions and Bits', run: a.connect } satisfies MenuEntry]
            : []),
        ],
      };
  return [
    { label: 'Manage accounts', icon: 'user', run: a.onmanage },
    'separator',
    // V109-C04 Your channel's clips, videos, followers, roles, rewards and moderation.
    { label: 'Creator dashboard', icon: 'layout', run: a.ondashboard },
    twitch,
    'separator',
    // V1051-02 No lock: the view explains itself.
    { label: '7TV paint and badge', icon: 'seventv', run: a.oncosmetics },
    // V109-B14
    { label: 'cyex cosmetics', icon: 'cyex', run: a.oncyex },
  ];
}
