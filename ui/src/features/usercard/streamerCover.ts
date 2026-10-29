// V108-C09 Streamer mode blurs the usercard picture until it is clicked. The reveal is
// never saved: it counts for one picture (Twitch or 7TV) of one user in one opening of
// the card, so anything else that changes blurs it again. The card also drops the reveal
// on a switch or a new opening, so switching back never brings an old reveal back.

export interface Shown {
  /** The user's id. */
  user: string;
  source: 'twitch' | '7tv';
  /** Goes up each time the card is opened again. */
  opening: number;
}

/** What a reveal is for. */
export const coverKey = (s: Shown): string => `${s.opening}:${s.user}:${s.source}`;

/** Whether the picture is blurred: streamer mode hides it, there is one, and the click
 *  that revealed it was for this very picture. */
export const covered = (hides: boolean, hasPicture: boolean, revealed: string, key: string): boolean =>
  hides && hasPicture && revealed !== key;
