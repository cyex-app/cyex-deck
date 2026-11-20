// V112-14 The cyex.tv sign-in running in this window, shared by the account cards and the
// cosmetics page. One at a time; starting another cancels the first (Rust does too).

import { cyextvIpc } from '../../ipc/cyextv';
import { errorText, rawError } from '../../lib/errors';
import { toast } from '../../lib/toast.svelte';

class TvSignIn {
  /** The account waiting for cyex.tv's page. */
  waiting = $state<string | null>(null);

  /** True once signed in. Cancelled stays quiet. */
  async start(account: string): Promise<boolean> {
    this.waiting = account;
    try {
      const name = await cyextvIpc.signIn(account);
      toast.success(name ? `Signed in to cyex.tv as ${name}.` : 'Signed in to cyex.tv.');
      return true;
    } catch (e) {
      if (rawError(e)) toast.error(errorText(e));
      return false;
    } finally {
      if (this.waiting === account) this.waiting = null;
    }
  }

  cancel() {
    void cyextvIpc.signInCancel();
  }
}

export const tvSignIn = new TvSignIn();
