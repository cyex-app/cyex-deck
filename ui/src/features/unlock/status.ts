// V108-7U What the sign-in dialog says for each step and state. Pure, so it's unit tested.

import type { SevenTvStage } from '../../ipc/bindings/SevenTvStage';
import type { SignIn } from '../../ipc/bindings/SignIn';
import type { Part } from './choice';

export type StepState = 'waiting' | 'running' | 'done' | 'failed';

export interface Status {
  lead: string;
  detail: string;
}

export interface Context {
  /** V108-7S/7U Where 7TV's window is: hidden first, shown, or past Twitch. */
  stage?: SevenTvStage | null;
  /** Why it failed, when there's more to say. */
  note?: string;
  /** Twitch's code, while one waits for approval. */
  code?: string | null;
  /** Twitch's sign-in page is open. */
  opened?: boolean;
}

export function statusOf(part: Part, state: StepState, c: Context = {}): Status {
  if (part === 'seventv') {
    if (state === 'done') return { lead: '7TV is connected.', detail: '' };
    if (state === 'failed') return { lead: 'Couldn’t connect 7TV.', detail: c.note || 'The 7TV window closed before it finished.' };
    if (state === 'waiting') return { lead: 'Connecting 7TV next.', detail: '' };
    if (c.stage === 'window') {
      return { lead: 'Finish signing in in the 7TV window.', detail: 'Sign in with Twitch and allow 7TV. It closes by itself.' };
    }
    if (c.stage === 'connecting') return { lead: 'Connecting to 7TV…', detail: 'Checking it’s your Twitch account.' };
    if (c.stage === 'hidden') return { lead: 'Connecting to 7TV…', detail: 'With your Twitch sign-in. This takes a few seconds.' };
    return { lead: 'Opening the 7TV window…', detail: '' };
  }
  if (state === 'done') return { lead: 'Subscriptions and wallet are unlocked.', detail: '' };
  if (state === 'failed') return { lead: 'Couldn’t unlock subscriptions and wallet.', detail: c.note || 'The sign-in didn’t finish.' };
  if (state === 'waiting') return { lead: 'Subscriptions and wallet next.', detail: '' };
  if (c.code) return { lead: `Approve ${c.code} in the Twitch window.`, detail: 'Then come back here.' };
  if (c.opened) return { lead: 'Finish signing in on Twitch.', detail: '' };
  return { lead: 'Opening Twitch…', detail: '' };
}

/** Why 7TV's sign-in didn't finish, for the dialog; '' means the window closed first. */
export function sevenTvNote(r: SignIn, name: string): string | null {
  if (r.type === 'done') return null;
  if (r.type === 'otherAccount') return `That’s a different Twitch account than ${name}.`;
  if (r.type === 'failed') return 'Couldn’t reach 7TV. Try again in a moment.';
  return '';
}

/** The run's own line once every step is through. */
export function doneLead(parts: Part[]): string {
  return parts.length > 1 ? 'All set.' : statusOf(parts[0] ?? 'seventv', 'done').lead;
}
