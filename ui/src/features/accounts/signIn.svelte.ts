// The Device Code sign-in running in this window, shared by the account cards and the
// add-account buttons. Starting a new one cancels the old one.

import { accountsIpc } from '../../ipc/accounts';
import type { DeviceStart } from '../../ipc/bindings/DeviceStart';
import type { TokenKind } from '../../ipc/bindings/TokenKind';
import { toast } from '../../lib/toast.svelte';

class SignInFlow {
  kind = $state<TokenKind | null>(null);
  code = $state<DeviceStart | null>(null);
  /** When the code arrived, for the countdown. */
  codeAt = $state(0);
  private run = 0;

  async start(kind: TokenKind): Promise<void> {
    const run = ++this.run;
    this.kind = kind;
    this.code = null;
    try {
      const name = await accountsIpc.signIn(kind, (code) => {
        if (run !== this.run) return;
        this.code = code;
        this.codeAt = Date.now();
      });
      toast.info(`Signed in as ${name}.`);
    } catch (e) {
      if (run === this.run) toast.error(String(e));
    } finally {
      if (run === this.run) {
        this.kind = null;
        this.code = null;
      }
    }
  }

  cancel(): void {
    this.run++;
    this.kind = null;
    this.code = null;
    void accountsIpc.cancelSignIn();
  }
}

export const signIn = new SignInFlow();
