// V107-N12a The counter counts what the send counts (app/src/chat/send.rs): a /me line is
// its text only, and Twitch's ACTION wrapper (9 chars) comes off its 500.
// V107-H9b A /me line that goes out as a cheer keeps the full 500 (the web client's rule);
// only a plain /me chat line loses the wrapper.

export const MAX_CHARS = 500;
const ACTION_WRAPPER = 9;

/** Characters that count and the limit for this text. `cheering`: the line takes the cheer path. */
export function sendLength(text: string, cheering = false): { chars: number; max: number } {
  const t = text.trim();
  const rest = t.startsWith('/me ') || t.startsWith('.me ') ? t.slice(4).trim() : null;
  if (rest !== null) return { chars: [...rest].length, max: cheering ? MAX_CHARS : MAX_CHARS - ACTION_WRAPPER };
  return { chars: [...t].length, max: MAX_CHARS };
}
