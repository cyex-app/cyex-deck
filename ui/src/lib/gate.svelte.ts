// V102-20 A greyed-out tool's flag takes you to the sign-in that unlocks it, from any
// window: Rust brings the main window up and Settings → Accounts shows that sign-in.
// V111-47 The ask names the account that lacks it, so the sign-in lands on that account's own
// row. `failToast` gives an error that says a sign-in is missing an Add button for it.

import { accountsIpc } from '../ipc/accounts';
import type { TokenKind } from '../ipc/bindings/TokenKind';
import { accounts } from './accounts.svelte';
import { errorText, rawError } from './errors';
import { needsKind } from './gate';
import { toast } from './toast.svelte';

export interface Want {
  kind: TokenKind;
  /** The account that lacks it; `null` when nobody is signed in. */
  userId: string | null;
}

class Gate {
  /** The sign-in asked for, until Accounts has shown it. */
  want = $state<Want | null>(null);
  /** Counts asks, so asking twice for the same one shows it again. */
  asks = $state(0);
  /** The account row Accounts is pointing at for a moment. */
  lit = $state<Want | null>(null);

  /** `userId`: the account the tool is used as (the default account when left out). */
  ask(kind: TokenKind, userId?: string | null): void {
    const who = userId ?? accounts.current?.userId ?? null;
    accountsIpc.showSignIn(kind, who).catch((e) => toast.error(errorText(e, 'open the sign-in')));
  }

  /** In the main window, when Rust passes an ask on. */
  shown(kind: TokenKind, userId: string | null = null): void {
    this.want = { kind, userId };
    this.asks++;
  }

  /** An error toast; when the error says a sign-in is missing, with an Add button for it. */
  fail(e: unknown, userId?: string | null, action?: string): void {
    const kind = needsKind(rawError(e));
    const text = errorText(e, action);
    if (kind) toast.error(text, { action: { label: 'Add', run: () => this.ask(kind, userId) } });
    else toast.error(text);
  }
}

export const gate = new Gate();
