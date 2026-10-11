// V101-08 The manual token screen, with Chatterino's fields. A whole Chatterino login line
// (`username=…;user_id=…;client_id=…;oauth_token=…`) pasted into any field fills them all.

import type { ManualToken } from '../../ipc/bindings/ManualToken';

const KEYS: Record<string, keyof ManualToken> = {
  username: 'login',
  login: 'login',
  user_id: 'userId',
  client_id: 'clientId',
  oauth_token: 'token',
  access_token: 'token',
};

/** The fields in a login line, or null when the text isn't one. */
export function parseLoginLine(text: string): Partial<ManualToken> | null {
  const out: Partial<ManualToken> = {};
  for (const part of text.split(/[;&\n]/)) {
    const eq = part.indexOf('=');
    if (eq < 0) continue;
    const key = KEYS[part.slice(0, eq).trim().toLowerCase()];
    const value = part.slice(eq + 1).trim();
    if (key && value) out[key] = value;
  }
  return Object.keys(out).length >= 2 ? out : null;
}
