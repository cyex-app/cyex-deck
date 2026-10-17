// V1061-03 Live players on the broadcast's VOD now, by channel login: chat says it's live (Q23), the stream side keeps them after the stream ends (Q21).
// Each player writes its own entry; the map folds them per login, so two players of one channel don't undo each other.
import { SvelteMap } from 'svelte/reactivity';

import { fold, type Rewound } from './rewoundFold.ts';

export const rewound = new SvelteMap<string, Rewound>();

const own = new Map<object, { login: string; v: Rewound }>();

function refold(login: string): void {
  const v = fold([...own.values()].filter((x) => x.login === login).map((x) => x.v));
  const cur = rewound.get(login);
  if (!v) {
    if (cur) rewound.delete(login);
  } else if (cur?.behind !== v.behind || cur.ended !== v.ended) rewound.set(login, v);
}

/** This player (`owner`) is on the VOD. */
export function setRewound(owner: object, login: string, v: Rewound): void {
  const e = own.get(owner);
  if (e && e.login === login && e.v.behind === v.behind && e.v.ended === v.ended) return;
  own.set(owner, { login, v });
  if (e && e.login !== login) refold(e.login);
  refold(login);
}

/** Only what this player wrote goes. */
export function clearRewound(owner: object): void {
  const e = own.get(owner);
  if (!e) return;
  own.delete(owner);
  refold(e.login);
}

export function hasOwn(owner: object): boolean {
  return own.has(owner);
}
