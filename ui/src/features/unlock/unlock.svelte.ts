// One question before a sign-in: 7TV, subscriptions and wallet, or both (the user's request,
// 2026/10/16). Then the chosen sign-ins run one after another in the same dialog. One per
// window; the dialog lives in App and PanelApp.

import { cosmeticsIpc } from '../../ipc/cosmetics';
import { accounts } from '../../lib/accounts.svelte';
import { toast } from '../../lib/toast.svelte';
import { signIn } from '../accounts/signIn.svelte';
import { doneOf, leftText, order, plan, sharedProfile, twitchLeft, type Entry, type Item, type Part } from './choice';

export type StepState = 'waiting' | 'running' | 'done' | 'failed';

export interface Step {
  part: Part;
  state: StepState;
  /** Why it failed, when there's more to say than "didn’t finish". */
  note?: string;
}

interface Asked {
  entry: Entry;
  account: string;
  name: string;
  /** `null`: one thing left, so no tickboxes, only the consent. */
  items: Item[] | null;
  parts: Part[];
  /** 7TV has no yes from this account yet. */
  consent: boolean;
}

interface Hooks {
  /** Runs after the steps with the parts that worked, e.g. the pick that was tried. */
  after?: (done: Part[]) => void;
  /** Settings: subscriptions and wallet alone open the account's own options (code or token). */
  twitchAlone?: () => void;
  /** 7TV's session ran out: it counts as not done, though it's still stored. */
  again?: boolean;
}

class Unlock {
  asked = $state.raw<Asked | null>(null);
  steps = $state<Step[] | null>(null);
  private hooks: Hooks = {};

  /** Asks, or runs straight away when there's no choice and no consent to show. */
  ask(entry: Entry, account: string, hooks: Hooks = {}): void {
    if (this.asked) return;
    const a = accounts.list.find((x) => x.userId === account);
    if (!a) return;
    const done = doneOf(a);
    if (hooks.again) done.seventv = false;
    const p = plan(entry, done, a.seventv.consent);
    if (p.kind === 'none') {
      hooks.after?.(['seventv', 'twitch']);
      return;
    }
    this.hooks = hooks;
    this.asked = {
      entry,
      account,
      name: a.displayName,
      items: p.kind === 'ask' ? p.items : null,
      parts: p.kind === 'run' ? p.parts : [],
      consent: !a.seventv.consent,
    };
    if (p.kind === 'run' && !p.consent) void this.go(p.parts);
  }

  get running(): boolean {
    return this.steps?.some((s) => s.state === 'running' || s.state === 'waiting') ?? false;
  }

  close(): void {
    if (this.running) return;
    this.asked = null;
    this.steps = null;
    this.hooks = {};
  }

  /** Stops a Twitch code that's waiting; the 7TV window is closed by the user. */
  stop(): void {
    if (signIn.kind) signIn.cancel();
  }

  async go(parts: Part[]): Promise<void> {
    const q = this.asked;
    if (!q || this.steps) return;
    const hooks = this.hooks;
    if (hooks.twitchAlone && parts.length === 1 && parts[0] === 'twitch') {
      this.close();
      hooks.twitchAlone();
      return;
    }
    this.steps = order(parts).map((part) => ({ part, state: 'waiting' }));
    const done: Part[] = [];
    const failed: Part[] = [];
    for (const step of this.steps) {
      step.state = 'running';
      const note =
        step.part === 'twitch' ? await this.twitch(q.account) : await this.seventv(q.account, sharedProfile(parts));
      step.state = note === null ? 'done' : 'failed';
      if (note) step.note = note;
      (note === null ? done : failed).push(step.part);
    }
    const notes = this.steps.flatMap((s) => (s.note ? [s.note] : []));
    this.asked = null;
    this.steps = null;
    this.hooks = {};
    if (failed.length) toast.error([...notes, leftText(done, failed)].join(' '));
    else toast.success(done.length > 1 ? 'All set.' : 'Signed in.');
    hooks.after?.(done);
  }

  /** Twitch's sign-ins still missing, each a code approved in the account's Twitch window. */
  private async twitch(account: string): Promise<string | null> {
    const a = accounts.list.find((x) => x.userId === account);
    if (!a) return '';
    for (const kind of twitchLeft(a)) {
      if (!(await signIn.start(kind, account, true))) return '';
    }
    return null;
  }

  /** The yes (the dialog's sign-in click, with the consent shown), then 7TV's own sign-in
   * window: in the account's Twitch profile when Twitch was picked too. */
  private async seventv(account: string, shared: boolean): Promise<string | null> {
    try {
      if (this.asked?.consent) await cosmeticsIpc.allow(account);
      const r = await cosmeticsIpc.signIn(account, shared);
      if (r.type === 'done') return null;
      if (r.type === 'otherAccount') return 'That’s a different Twitch account.';
      return '';
    } catch {
      return '';
    }
  }
}

export const unlock = new Unlock();
