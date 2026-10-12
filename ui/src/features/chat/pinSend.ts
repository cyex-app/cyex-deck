// M11 `/pin <text>` sends a message and pins it; `/pin <message id> [5m]` pins one that's there.

const MESSAGE_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** The text to send and pin, or null when the line isn't one. */
export function pinText(line: string): string | null {
  const m = /^\/pin\s+(.+)$/s.exec(line.trim());
  if (!m) return null;
  const text = m[1].trim();
  return MESSAGE_ID.test(text.split(/\s/)[0]) ? null : text;
}
