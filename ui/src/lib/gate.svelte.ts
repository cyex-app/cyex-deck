// V102-20 A greyed-out tool's flag takes you to the sign-in that unlocks it, from any
// window: Rust brings the main window up and Settings → Accounts shows that sign-in.

import { accountsIpc } from '../ipc/accounts';
import type { TokenKind } from '../ipc/bindings/TokenKind';
import { toast } from './toast.svelte';

class Gate {
  /** The sign-in asked for, until Accounts has shown it. */
  want = $state<TokenKind | null>(null);
  /** Counts asks, so asking twice for the same one shows it again. */
  asks = $state(0);

  ask(kind: TokenKind): void {
    accountsIpc.showSignIn(kind).catch((e) => toast.error(String(e)));
  }

  /** In the main window, when Rust passes an ask on. */
  shown(kind: TokenKind): void {
    this.want = kind;
    this.asks++;
  }
}

export const gate = new Gate();
