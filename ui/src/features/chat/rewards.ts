// V107-H11b Rewards the balance can't pay for: still clickable, but marked, and redeeming
// says so before Twitch is asked. Same words as Twitch's refusal (app/src/chat/points.rs).

export const NOT_ENOUGH = 'Not enough points';

/** `balance` unknown (signed out, not read yet): nothing is marked. */
export function short(cost: number, balance: number | null | undefined): boolean {
  return balance != null && cost > balance;
}

/** What redeeming answers before asking Twitch, or null to go ahead. */
export function blocked(cost: number, balance: number | null | undefined): string | null {
  return short(cost, balance) ? `${NOT_ENOUGH}.` : null;
}
