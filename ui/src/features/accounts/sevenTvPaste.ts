// V108-7K "Paste a 7TV token": what the how-to copies, and a first look at the paste before
// anything is sent (Rust checks it again, and 7TV has the last word).

/** Run in 7tv.app's console: copies the session without showing it. */
export const CONSOLE_LINE = "copy(localStorage.getItem('7tv-token'))";

/** Three base64url parts, at most 1.200 chars, optionally in the quotes the console shows. */
export function looksLikeToken(raw: string): boolean {
  let t = raw.trim();
  if (t.length > 1 && (t[0] === '"' || t[0] === "'") && t.at(-1) === t[0]) t = t.slice(1, -1);
  if (t.length > 1200) return false;
  const parts = t.split('.');
  return parts.length === 3 && parts.every((p) => /^[A-Za-z0-9_-]+$/.test(p));
}
