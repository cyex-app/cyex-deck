// S7, S10 How plugins' pieces look and what their clicks say. A tip on a clickable piece
// always says what the click does, so a plugin can't hide it.

import type { Act } from '../../ipc/bindings/Act';
import type { SpanColor } from '../../ipc/bindings/SpanColor';

const MAX_SHOWN = 60;
/** S10 A picture is at most a giant emote's height, and 4 times as wide. */
const MAX_HEIGHT = 112;
const MAX_WIDTH = MAX_HEIGHT * 4;

const cut = (s: string) => (s.length > MAX_SHOWN ? `${s.slice(0, MAX_SHOWN)}…` : s);

/** What a click does, in a few words. URLs show as links with their real host instead. */
export function actTip(a: Act): string {
  switch (a.kind) {
    case 'run':
      return a.text.startsWith('/') ? `Runs ${cut(a.text)}` : `Sends ${cut(a.text)}`;
    case 'user':
      return `${a.login}’s usercard`;
    case 'jump':
      return 'Go to the line';
    case 'copy':
      return `Copies ${cut(a.text)}`;
    case 'insert':
      return `Puts ${cut(a.text)} in the message box`;
    case 'url':
      return a.host;
  }
}

/** A span's color as a class and the readable colors for a custom one. */
export function spanColor(c: SpanColor | undefined): { cls: string; vars: string } {
  if (!c) return { cls: '', vars: '' };
  if (c === 'link') return { cls: 'c-link', vars: '' };
  if (c === 'system') return { cls: 'c-system', vars: '' };
  return { cls: 'c-rgb', vars: `--cd:${c.rgb.dark};--cl:${c.rgb.light}` };
}

/** The size a picture shows at: its own, made to fit the line's limits. */
export function picSize(size: [number, number] | undefined): { width: number; height: number } | null {
  if (!size) return null;
  const [w, h] = size;
  const scale = Math.min(1, MAX_HEIGHT / h, MAX_WIDTH / w);
  return { width: Math.round(w * scale), height: Math.round(h * scale) };
}
