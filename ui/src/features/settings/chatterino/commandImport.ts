// V111-06 Chatterino's commands: what an import with these choices does (mirrors
// `chatterino::commands::apply`), and the text for it.

import type { CommandImport } from '../../../ipc/bindings/CommandImport';
import type { CommandItem } from '../../../ipc/bindings/CommandItem';
import type { CommandPreview } from '../../../ipc/bindings/CommandPreview';
import { formatNumber } from '../../../lib/format.ts';

export interface Outcome {
  added: number;
  replaced: number;
  kept: number;
  /** Names that don't fit under the limit. */
  cut: string[];
  /** Message-menu entries that come along. */
  menu: number;
}

export const isConflict = (i: CommandItem) => i.conflict !== 'none';

/** `take`: ids of conflicts that take Chatterino's. */
export function outcome(p: CommandPreview, take: ReadonlySet<number>): Outcome {
  const o: Outcome = { added: 0, replaced: 0, kept: 0, cut: [], menu: 0 };
  for (const i of p.items) {
    if (isConflict(i) && !take.has(i.id)) {
      o.kept++;
      continue;
    }
    if (i.conflict === 'yours') o.replaced++;
    else if (o.added < p.room) o.added++;
    else {
      o.cut.push(i.name);
      continue;
    }
    if (i.menu) o.menu++;
  }
  return o;
}

const plural = (n: number, one: string, many = `${one}s`) => `${formatNumber(n)} ${n === 1 ? one : many}`;

/** "12 commands, 3 already there." */
export function headline(p: CommandPreview): string {
  const n = p.items.length;
  if (!n) return 'Nothing to import.';
  const clash = p.items.filter(isConflict).length;
  return clash ? `${plural(n, 'command')}, ${formatNumber(clash)} with a name in use.` : `${plural(n, 'command')}.`;
}

/** "Limit is 500. 2 won’t fit: /a, /b." or null. */
export function limitText(p: CommandPreview, o: Outcome, shown = 10): string | null {
  if (!o.cut.length) return null;
  const names = o.cut.slice(0, shown).join(', ');
  const more = o.cut.length > shown ? ` and ${formatNumber(o.cut.length - shown)} more` : '';
  return `Limit is ${formatNumber(p.limit)}. ${plural(o.cut.length, 'command')} won’t fit: ${names}${more}.`;
}

/** Toast after the import: "Imported 9, replaced 2, 3 in the message menu." */
export function resultText(r: CommandImport): string {
  const parts: string[] = [];
  if (r.added) parts.push(`Imported ${formatNumber(r.added)}`);
  if (r.replaced) parts.push(`${parts.length ? 'replaced' : 'Replaced'} ${formatNumber(r.replaced)}`);
  if (r.menu) parts.push(`${formatNumber(r.menu)} in the message menu`);
  if (r.cut.length) parts.push(`${formatNumber(r.cut.length)} over the limit`);
  return parts.length ? `${parts.join(', ')}.` : 'Nothing new.';
}
