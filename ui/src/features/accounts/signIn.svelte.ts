// The Device Code sign-in running in this window, shared by the account cards and the
// add-account buttons. Starting a new one cancels the old one.

import { accountsIpc } from '../../ipc/accounts';
import type { DeviceStart } from '../../ipc/bindings/DeviceStart';
import type { TokenKind } from '../../ipc/bindings/TokenKind';
import { toast } from '../../lib/toast.svelte';
import { errorText } from '../../lib/errors';

class SignInFlow {
  kind = $state<TokenKind | null>(null);
  /** The account the running sign-in is added to; `null` for any account. */
  forUser = $state<string | null>(null);
  code = $state<DeviceStart | null>(null);
  /** When the code arrived, for the countdown. */
  codeAt = $state(0);
  /** V102-19 Right after Sign in with Twitch: the account offered the GQL tools. */
  offer = $state<string | null>(null);
  private run = 0;

  async start(kind: TokenKind, userId: string | null = null): Promise<void> {
    const run = ++this.run;
    this.kind = kind;
    this.forUser = userId;
    this.code = null;
    try {
      const who = await accountsIpc.signIn(kind, userId, (code) => {
        if (run !== this.run) return;
        this.code = code;
        this.codeAt = Date.now();
      });
      toast.success(`Signed in as ${who.displayName}.`);
      this.offer = kind === 'helix' ? who.userId : null;
    } catch (e) {
      if (run === this.run) toast.error(errorText(e));
    } finally {
      if (run === this.run) {
        this.kind = null;
        this.forUser = null;
        this.code = null;
      }
    }
  }

  cancel(): void {
    this.run++;
    this.kind = null;
    this.forUser = null;
    this.code = null;
    void accountsIpc.cancelSignIn();
  }
}

export const signIn = new SignInFlow();
