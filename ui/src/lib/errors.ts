// Backend errors as short human text. IPC calls reject with the Rust error's text
// (`e.to_string()`); this turns the Twitch client's raw shapes into a sentence and leaves the
// app's own messages as they are, tidied. Ready for a typed `{ kind, msg }` from Rust too.
//
//   .catch((e) => toast.error(errorText(e)))
//   .catch((e) => toast.error(errorText(e, 'save the settings')))   // "Couldn’t save the settings."

export type ErrorKind =
  | 'network' // offline, DNS, timeout
  | 'auth' // 401: the sign-in ran out
  | 'forbidden' // 403
  | 'missing' // 404
  | 'rate' // 429
  | 'server' // 5xx
  | 'refused' // GQL said no with a code
  | 'decode' // an answer we couldn't read
  | 'internal' // a bug or a bad call, not for the user's eyes
  | 'message'; // already a sentence for the user

/** The raw text of anything thrown or rejected. */
export function rawError(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e && typeof e === 'object') {
    const o = e as { msg?: unknown; message?: unknown };
    if (typeof o.msg === 'string') return o.msg;
    if (typeof o.message === 'string') return o.message;
  }
  return '';
}

const KINDS = new Set<ErrorKind>([
  'network',
  'auth',
  'forbidden',
  'missing',
  'rate',
  'server',
  'refused',
  'decode',
  'internal',
  'message',
]);

const API = /^Twitch said: ?(.*?) ?\((\d{3})\)$/s;
const REFUSED = /^Twitch said no(?:: | \()([A-Z0-9_]+)\)?\.?$/;

export function errorKind(e: unknown): ErrorKind {
  const kind = (e as { kind?: unknown } | null)?.kind;
  if (typeof kind === 'string' && KINDS.has(kind as ErrorKind)) return kind as ErrorKind;
  const raw = rawError(e).trim();
  if (!raw) return 'internal';
  if (/^couldn[’']t reach Twitch/i.test(raw) || /failed to fetch|network ?error|timed? ?out/i.test(raw)) return 'network';
  const api = API.exec(raw);
  if (api) {
    const status = Number(api[2]);
    if (status === 401) return 'auth';
    if (status === 403) return 'forbidden';
    if (status === 404) return 'missing';
    if (status === 429) return 'rate';
    if (status >= 500) return 'server';
    return 'message';
  }
  if (REFUSED.test(raw)) return 'refused';
  if (/^unexpected answer from Twitch/i.test(raw)) return 'decode';
  // Tauri's own: a command called with the wrong arguments, or one that doesn't exist.
  if (/^(invalid args|command \S+ not found|missing required key)/i.test(raw)) return 'internal';
  return 'message';
}

const FIXED: Partial<Record<ErrorKind, string>> = {
  network: 'Couldn’t reach Twitch. Check your connection.',
  auth: 'Your sign-in ran out. Sign in again.',
  forbidden: 'Twitch won’t allow that.',
  missing: 'Twitch couldn’t find that.',
  rate: 'Too many tries. Wait a moment.',
  server: 'Twitch didn’t answer. Try again in a bit.',
  decode: 'Couldn’t read Twitch’s answer.',
};

/** GQL refusal codes worth their own words; the rest read "Twitch said no." */
const REFUSALS: Record<string, string> = {
  NOT_ENOUGH_POINTS: 'Not enough channel points.',
  INSUFFICIENT_COMMUNITY_POINTS_BALANCE: 'Not enough channel points.',
  RATE_LIMITED: 'Slow down a little.',
  FORBIDDEN: 'Twitch won’t allow that.',
  UNAUTHORIZED: 'Your sign-in can’t do that.',
};

/** One short sentence for the user. `action` ("save the settings") words the cases that
 *  have nothing better to say: "Couldn’t save the settings." */
export function errorText(e: unknown, action?: string): string {
  const kind = errorKind(e);
  const raw = rawError(e).trim();
  const fallback = action ? `Couldn’t ${action}.` : 'Something went wrong. Try again.';
  if (kind === 'internal') return fallback;
  if (kind === 'refused') {
    const code = REFUSED.exec(raw)?.[1] ?? '';
    return REFUSALS[code] ?? 'Twitch said no.';
  }
  const fixed = FIXED[kind];
  if (fixed) return fixed;
  const api = API.exec(raw);
  return sentence(api ? `Twitch said: ${api[1]}` : raw) || fallback;
}

/** Capital first letter, curly apostrophes, a full stop. A line that already ends in one is a
 *  finished sentence from Rust and keeps its first letter ("mockme has no 7TV emote…"). A line
 *  ending in ")" is library text ("… (os error 5)"): capital letter, no full stop added. */
export function sentence(text: string): string {
  const t = text.trim().replace(/(\w)'(\w)/g, '$1’$2');
  if (!t) return '';
  // K10 A finished line from Rust keeps its first letter: it may start with a login.
  if (/[.!?…]$/.test(t)) return t;
  const s = `${t[0].toUpperCase()}${t.slice(1)}`;
  return s.endsWith(')') ? s : `${s}.`;
}
