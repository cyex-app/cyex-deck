// V111-60 Settings › Import: text for each kind's review and result, and the one Undo for an
// import of several kinds. V105-16's highlight texts moved here from HighlightImport.

import type { Chat } from '../../../ipc/bindings/Chat';
import type { Commands } from '../../../ipc/bindings/Commands';
import type { ExtrasPreview } from '../../../ipc/bindings/ExtrasPreview';
import type { HighlightImport } from '../../../ipc/bindings/HighlightImport';
import type { HighlightPreview } from '../../../ipc/bindings/HighlightPreview';
import type { Highlights } from '../../../ipc/bindings/Highlights';
import type { ImportKind } from '../../../ipc/bindings/ImportKind';
import type { UserNote } from '../../../ipc/bindings/UserNote';
import { formatNumber } from '../../../lib/format.ts';

const plural = (n: number, one: string, many = `${one}s`) => `${formatNumber(n)} ${n === 1 ? one : many}`;

export const KINDS: readonly { kind: ImportKind; label: string; hint: string }[] = [
  { kind: 'tabs', label: 'Tabs', hint: 'Your tabs and channels, in order and with their names.' },
  { kind: 'highlights', label: 'Highlights', hint: 'Words, people and badges that highlight.' },
  { kind: 'commands', label: 'Commands', hint: 'Your own commands, and the ones on right-click.' },
  { kind: 'extras', label: 'Ignores, nicknames and filters', hint: 'Ignored words, never highlight, muted chats, filters and nicknames.' },
];

/** What can't come from Chatterino, and why. */
export const LEFT_OUT: readonly { label: string; text: string }[] = [
  { label: 'Accounts', text: 'Sign in again here. Chatterino’s tokens are never read.' },
  { label: 'Blocked people', text: 'They come with your Twitch account.' },
  { label: 'Hotkeys', text: 'The actions differ. Set them in Hotkeys.' },
  { label: 'Mod buttons', text: 'Pick timeouts and buttons in Moderation.' },
  { label: 'Look and other settings', text: 'Names and options differ, so they start fresh.' },
  { label: 'Chat logs', text: 'They stay in Chatterino’s folder.' },
];

/** "3 new: 2 words, 1 person." */
export function highlightHeadline(p: HighlightPreview): string {
  const fresh = p.phrases + p.users + p.badges;
  if (!fresh) return 'Nothing new to import.';
  const parts = [
    p.phrases && plural(p.phrases, 'word'),
    p.users && plural(p.users, 'person', 'people'),
    p.badges && plural(p.badges, 'badge'),
  ].filter(Boolean);
  return `${formatNumber(fresh)} new: ${parts.join(', ')}.`;
}

export function highlightCanImport(p: HighlightPreview, choice: 'mine' | 'theirs'): boolean {
  return p.phrases + p.users + p.badges > 0 || (p.duplicates.length > 0 && choice === 'theirs');
}

/** "Imported 3, skipped 1." */
export function highlightResult(r: HighlightImport): string {
  const n = r.added + r.replaced;
  const skipped = r.skipped.length ? `, skipped ${formatNumber(r.skipped.length)}` : '';
  return n ? `Imported ${formatNumber(n)}${skipped}.` : `Nothing new${skipped}.`;
}

/** The kinds' new rows as "2 ignores, 1 nickname". */
export function extrasParts(p: ExtrasPreview): string[] {
  return [
    p.ignores && plural(p.ignores, 'ignore'),
    p.never && `${formatNumber(p.never)} never highlight`,
    p.muted && plural(p.muted, 'muted chat'),
    p.filters && plural(p.filters, 'filter'),
    p.nicknames && plural(p.nicknames, 'nickname'),
  ].filter((x): x is string => !!x);
}

export const extrasCount = (p: ExtrasPreview) => p.ignores + p.never + p.muted + p.filters + p.nicknames;

/** "5 new: 2 ignores, 3 filters." */
export function extrasHeadline(p: ExtrasPreview): string {
  const n = extrasCount(p);
  return n ? `${formatNumber(n)} new: ${extrasParts(p).join(', ')}.` : 'Nothing new to import.';
}

/** The partitions an import can change, as they were before it. */
export interface Before {
  highlights: Pick<Highlights, 'phrases' | 'users' | 'badges' | 'ignores' | 'never' | 'muted' | 'filters'>;
  commands: Pick<Commands, 'lines' | 'messageMenu'>;
  users: Chat['users'];
}

export function snapshot(h: Highlights, c: Commands, chat: Chat): Before {
  return {
    highlights: {
      phrases: [...h.phrases],
      users: [...h.users],
      badges: [...h.badges],
      ignores: [...h.ignores],
      never: [...h.never],
      muted: [...h.muted],
      filters: [...h.filters],
    },
    commands: { lines: [...c.lines], messageMenu: [...c.messageMenu] },
    users: { ...chat.users },
  };
}

/** The merge patch that puts people notes back: old ones as they were, new ones gone. */
export function usersUndo(before: Chat['users'], now: Chat['users']): Record<string, UserNote | null> {
  const patch: Record<string, UserNote | null> = { ...before };
  for (const id of Object.keys(now)) if (!(id in before)) patch[id] = null;
  return patch;
}

export interface Done {
  tabs: number;
  highlights: number;
  commands: number;
  extras: number;
  failed: string[];
}

/** One toast for the whole import: "Imported 2 tabs, 5 highlights and 3 commands." */
export function doneText(d: Done): string {
  const parts = [
    d.tabs && plural(d.tabs, 'tab'),
    d.highlights && plural(d.highlights, 'highlight'),
    d.commands && plural(d.commands, 'command'),
    d.extras && `${formatNumber(d.extras)} more`,
  ].filter((x): x is string => !!x);
  const list = parts.length > 1 ? `${parts.slice(0, -1).join(', ')} and ${parts.at(-1)}` : (parts[0] ?? '');
  const head = list ? `Imported ${list}.` : 'Nothing new.';
  return d.failed.length ? `${head} ${d.failed.join(' ')}` : head;
}
