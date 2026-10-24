// The sign-in running in this window, shared by the account cards and the add-account
// buttons. Starting a new one cancels the old one. V108-01 "Sign in with Twitch" runs on
// Twitch's sign-in page; "Sign in with a code" shows a code. V108-02 Both open where the
// user picked: the browser or a fresh app window.

import { accountsIpc } from '../../ipc/accounts';
import type { DeviceStart } from '../../ipc/bindings/DeviceStart';
import type { Place } from '../../ipc/bindings/Place';
import type { TokenKind } from '../../ipc/bindings/TokenKind';
import { accounts } from '../../lib/accounts.svelte';
import { toast } from '../../lib/toast.svelte';
import { errorText } from '../../lib/errors';
import { placeFor, readPlace, savePlace } from './place';

class SignInFlow {
  kind = $state<TokenKind | null>(null);
  /** The account the running sign-in is added to; `null` for any account. */
  forUser = $state<string | null>(null);
  code = $state<DeviceStart | null>(null);
  /** When the code arrived, for the countdown. */
  codeAt = $state(0);
  /** V108-01 Twitch's sign-in page is open, waiting for the user. */
  opened = $state(false);
  /** Where the running sign-in opened. */
  at = $state<Place>('browser');
  /** V102-19 Right after Sign in with Twitch: the account offered the GQL tools. */
  offer = $state<string | null>(null);
  /** V108-02 What the user picked last, if anything. */
  picked = $state<Place | null>(readPlace());
  private run = 0;

  /** Where a sign-in for `userId` opens. */
  placeFor(userId: string | null): Place {
    return placeFor(this.picked, userId, accounts.list.length);
  }

  pick(place: Place): void {
    this.picked = place;
    savePlace(place);
  }

  /** `true` once signed in. `quiet`: the caller shows how it went (no toasts, no GQL offer). */
  async start(kind: TokenKind, userId: string | null = null, quiet = false): Promise<boolean> {
    const run = ++this.run;
    const place = this.placeFor(userId);
    this.kind = kind;
    this.forUser = userId;
    this.code = null;
    this.opened = false;
    this.at = place;
    try {
      const who =
        kind === 'helix'
          ? await accountsIpc.signInBrowser(userId, place, () => {
              if (run === this.run) this.opened = true;
            })
          : await accountsIpc.signIn(kind, userId, place, (code) => {
              if (run !== this.run) return;
              this.code = code;
              this.codeAt = Date.now();
            });
      if (quiet) return true;
      toast.success(`Signed in as ${who.displayName}.`);
      this.offer = kind === 'helix' ? who.userId : null;
      return true;
    } catch (e) {
      if (run === this.run && !quiet) toast.error(errorText(e));
      return false;
    } finally {
      if (run === this.run) this.reset();
    }
  }

  cancel(): void {
    this.run++;
    this.reset();
    void accountsIpc.cancelSignIn();
  }

  private reset(): void {
    this.kind = null;
    this.forUser = null;
    this.code = null;
    this.opened = false;
  }
}

export const signIn = new SignInFlow();
