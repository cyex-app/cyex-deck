// V107-N12a The counter counts what the send counts (app/src/chat/send.rs): a /me line is
// its text only, and Twitch's ACTION wrapper (9 chars) comes off its 500.

export const MAX_CHARS = 500;
const ACTION_WRAPPER = 9;

/** Characters that count and the limit for this text. */
export function sendLength(text: string): { chars: number; max: number } {
  const t = text.trim();
  const rest = t.startsWith('/me ') || t.startsWith('.me ') ? t.slice(4).trim() : null;
  if (rest !== null) return { chars: [...rest].length, max: MAX_CHARS - ACTION_WRAPPER };
  return { chars: [...t].length, max: MAX_CHARS };
}
