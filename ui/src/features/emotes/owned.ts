// V107-F10 What Tab and `:` completion may offer: emotes the account can use here (the
// menu's unlocked ones, which the Rust side filtered by sub and follow) plus the
// third-party emotes seen in this chat. A Twitch emote seen in chat counts only when it
// is in the usable set: other people's sub emotes are not yours. Signed out, that leaves
// no Twitch emotes at all.

import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import { emoteKey } from './emotes.ts';
import type { EmoteName } from '../chat/suggestions.ts';
import { usable } from './menu.ts';

export function completionEmotes(menu: EmoteMenu | null, seen: Iterable<EmoteName>): EmoteName[] {
  const mine = menu ? usable(menu) : new Map();
  const out: EmoteName[] = [...mine.values()].map((e) => ({ name: e.name, provider: e.provider, id: e.id, animated: e.animated }));
  for (const s of seen) {
    if (s.provider === 'twitch' && !mine.has(emoteKey(s.provider, s.id))) continue;
    out.push(s);
  }
  return out;
}
