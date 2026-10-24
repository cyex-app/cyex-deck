// One question before a sign-in: 7TV, subscriptions and wallet, or both (the user's request,
// 2026/10/16). Then the chosen sign-ins run one after another in the same dialog. One per
// window; the dialog lives in App and PanelApp.
// V108-7U The dialog stays through the end: what's happening now, Cancel, Try again when
// something didn't finish, and 7TV from the user's browser (bookmark or paste) instead.

import { cosmeticsIpc } from '../../ipc/cosmetics';
import { accounts } from '../../lib/accounts.svelte';
import { alsoSevenTv } from '../accounts/alsoSevenTv.svelte';
import { signIn } from '../accounts/signIn.svelte';
import { doneOf, order, plan, sevenTvShared, twitchLeft, type Entry, type Item, type Part } from './choice';
import { sevenTvNote, type StepState } from './status';
import type { SevenTvStage } from '../../ipc/bindings/SevenTvStage';

export type { StepState };

export interface Step {
  part: Part;
  state: StepState;
  /** Why it failed, when there's more to say than "didn’t finish". */
  note?: string;
  /** V108-7U Where 7TV's window is, while it runs. */
  stage?: SevenTvStage | null;
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
  /** V108-7S 7TV in the account's Twitch pages profile (hidden first) whatever "Open in" says. */
  shared?: boolean;
}

/** How long "Connected" stays before the dialog closes by itself. */
const DONE_FOR = 1100;

class Unlock {
  asked = $state.raw<Asked | null>(null);
  steps = $state<Step[] | null>(null);
  /** V108-7U The run is over; the dialog shows how it went. */
  ended = $state(false);
  /** V108-7U 7TV from the user's browser: the bookmark and paste route, in the dialog. */
  browser = $state(false);
  private hooks: Hooks = {};
  /** Cancel was pressed: the run ends quietly. */
  private stopping = false;
  private told = false;
  private timer: ReturnType<typeof setTimeout> | undefined;

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
    // V108-7P 7TV offered here: the card won't ask "Also connect 7TV?" after.
    if ((p.kind === 'ask' && p.items.some((i) => i.part === 'seventv' && !i.done)) || (p.kind === 'run' && p.parts.includes('seventv'))) alsoSevenTv.seen(account);
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
    return !this.ended && (this.steps?.some((s) => s.state === 'running' || s.state === 'waiting') ?? false);
  }

  /** The step running now, else the first that didn't finish. */
  get current(): Step | null {
    const s = this.steps ?? [];
    return s.find((x) => x.state === 'running') ?? s.find((x) => x.state === 'failed') ?? null;
  }

  /** Closes the dialog once nothing runs. After a run, the parts that worked go to `after`. */
  close(): void {
    if (this.running) return;
    clearTimeout(this.timer);
    const steps = this.steps;
    const hooks = this.hooks;
    const tell = !!steps && !this.told;
    this.asked = null;
    this.steps = null;
    this.ended = false;
    this.browser = false;
    this.hooks = {};
    this.told = false;
    if (tell) hooks.after?.(steps.filter((s) => s.state === 'done').map((s) => s.part));
  }

  /** Stops what runs (Twitch's code, or 7TV's window), then closes without a word. */
  cancel(): void {
    if (!this.running) return this.close();
    this.stopping = true;
    const step = this.current;
    if (step?.part === 'seventv' && this.asked) void cosmeticsIpc.signInCancel(this.asked.account).catch(() => {});
    else if (signIn.kind) signIn.cancel();
  }

  /** V108-7U 7TV from the browser instead: the window goes, the dialog shows the route. */
  useBrowser(): void {
    this.browser = true;
    const step = this.current;
    if (this.running && step?.part === 'seventv' && this.asked) void cosmeticsIpc.signInCancel(this.asked.account).catch(() => {});
  }

  /** The bookmark or paste connected 7TV. */
  browserDone(): void {
    const step = this.steps?.find((s) => s.part === 'seventv');
    this.browser = false;
    if (!step) return this.close();
    step.state = 'done';
    step.note = undefined;
    // Still running: the run ends it once 7TV's window is gone.
    if (this.ended && this.steps?.every((x) => x.state === 'done')) this.finish();
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
    await this.run();
  }

  /** Runs the steps that didn't finish again. */
  async retry(): Promise<void> {
    if (!this.ended || !this.steps) return;
    for (const s of this.steps) {
      if (s.state === 'failed') {
        s.state = 'waiting';
        s.note = undefined;
      }
    }
    this.browser = false;
    await this.run();
  }

  private async run(): Promise<void> {
    const q = this.asked;
    if (!q || !this.steps) return;
    clearTimeout(this.timer);
    this.ended = false;
    this.stopping = false;
    const hooks = this.hooks;
    for (const step of this.steps) {
      if (step.state !== 'waiting') continue;
      if (this.stopping) {
        step.state = 'failed';
        continue;
      }
      step.state = 'running';
      step.stage = null;
      const note =
        step.part === 'twitch'
          ? await this.twitch(q.account)
          : await this.seventv(q.account, q.name, step, hooks.shared ?? sevenTvShared(signIn.place));
      // The bookmark may have connected 7TV while its window was still closing.
      if (step.state === 'running') step.state = note === null ? 'done' : 'failed';
      step.stage = null;
      if (note) step.note = note;
    }
    if (this.stopping) {
      this.stopping = false;
      this.ended = true;
      this.close();
      return;
    }
    if (this.steps.every((s) => s.state === 'done')) {
      if (!this.told) this.finish();
    }
    else this.ended = true;
  }

  /** All through: the parts go to `after` now, and the dialog closes a moment later. */
  private finish(): void {
    const steps = this.steps;
    if (!steps) return;
    this.ended = true;
    this.told = true;
    this.hooks.after?.(steps.map((s) => s.part));
    clearTimeout(this.timer);
    this.timer = setTimeout(() => this.close(), DONE_FOR);
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
   * window: V108-7T in the account's Twitch pages profile when "Open in" is Browser. */
  private async seventv(account: string, name: string, step: Step, shared: boolean): Promise<string | null> {
    const off = await cosmeticsIpc.onSignInStage((who, stage) => {
      if (who === account && step.state === 'running') step.stage = stage;
    });
    try {
      if (this.asked?.consent) {
        await cosmeticsIpc.allow(account);
        // Asked once: a second try doesn't show the consent again.
        if (this.asked) this.asked = { ...this.asked, consent: false };
      }
      return sevenTvNote(await cosmeticsIpc.signIn(account, shared), name);
    } catch {
      return '';
    } finally {
      off();
    }
  }
}

export const unlock = new Unlock();
