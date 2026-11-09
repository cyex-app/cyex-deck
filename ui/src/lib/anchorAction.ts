// V110-F83 Which clicks on an anchor the window-level guard stops (linkGuard.ts).

export type AnchorAction = 'ignore' | 'stop' | 'open';

/** What to do with a click that reached the window on an anchor. `href` is the resolved one. */
export function anchorAction(href: string, origin: string, prevented: boolean): AnchorAction {
  if (prevented) return 'ignore';
  let url: URL;
  try {
    url = new URL(href);
  } catch {
    return 'stop';
  }
  // Same page (`#`, hash links) or any route of the app itself.
  if (url.origin === origin && origin !== 'null') return 'ignore';
  return url.protocol === 'http:' || url.protocol === 'https:' ? 'open' : 'stop';
}
