/** F7 A Twitch login is 1 to 25 letters, digits or underscores. A row may hold "a, b" (Rust
 * splits it), so each piece is checked. Empty is fine: it's a row not filled yet. */
const LOGIN = /^[a-z0-9_]{1,25}$/;

export function loginProblem(text: string): string | null {
  const parts = text.split(/[\s,]+/).filter(Boolean);
  return parts.every((p) => LOGIN.test(p.replace(/^@/, '').toLowerCase())) ? null : 'Not a Twitch name.';
}
