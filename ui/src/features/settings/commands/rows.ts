// V106-10 Command and menu rows as two fields, lossless: a row nobody edited keeps its exact text.
import type { CommandRow } from '../../../ipc/bindings/CommandRow';

export interface CommandView extends Record<string, unknown> {
  enabled: boolean;
  name: string;
  sends: string;
  raw: string;
}
export interface MenuView extends Record<string, unknown> {
  enabled: boolean;
  label: string;
  command: string;
  raw: string;
}

/** Split at the first ' ' (as Rust does); the rest is kept as is. A leading space means an empty name. */
function splitSpace(text: string): [string, string] {
  const at = text.indexOf(' ');
  if (at < 0) return [text, ''];
  return [text.slice(0, at), text.slice(at + 1)];
}

/** "# note" and the like: a comment row. */
const isNote = (text: string) => text.trim().startsWith('#');

/** Split at the first "="; notes split at the first space; `@Copy`, `-`, `/Name` have no "=". */
function splitMenu(text: string): [string, string] {
  if (isNote(text)) return splitSpace(text);
  const at = text.indexOf('=');
  if (at < 0) return [text, ''];
  return [text.slice(0, at), text.slice(at + 1)];
}

/** "/hi Hello $1!" → name "/hi", sends "Hello $1!". "# note" → name "#", sends "note". */
export function commandView(r: CommandRow): CommandView {
  const [name, sends] = splitSpace(r.text);
  return { enabled: r.enabled, name, sends, raw: r.text };
}

/** V110-F90 Longest command name kept (with its "/"). */
export const NAME_MAX = 32;

/** V110-F90 A typed name: no spaces, a "/" in front (a "#" note keeps its mark), at most NAME_MAX. */
export function commandName(typed: string): string {
  const n = typed.replace(/\s+/g, '');
  if (!n) return n;
  return (n.startsWith('/') || n.startsWith('#') ? n : `/${n}`).slice(0, NAME_MAX);
}

/** V110-F90 What's wrong with a command row as written, or null. Blank rows and notes are fine. */
export function commandProblem(v: CommandView): string | null {
  const name = v.name.trim();
  if (name.startsWith('#') || (!name && !v.sends.trim())) return null;
  if (!name) return 'Give it a name.';
  return v.sends.trim() ? null : 'Add what it sends.';
}

/** Unchanged name and sends: the old text. Else name (spaces removed) + " " + sends, or just the name. */
export function commandRow(v: CommandView): CommandRow {
  const [name, sends] = splitSpace(v.raw);
  if (v.name === name && v.sends === sends) return { enabled: v.enabled, text: v.raw };
  const n = commandName(v.name);
  return { enabled: v.enabled, text: v.sends ? `${n} ${v.sends}` : n };
}

/** V110-F90 "-" draws a line and "@Name" opens a submenu: neither runs a command. */
const isStructural = (label: string) => label.trim() === '-' || label.trim().startsWith('@');

/** V110-F90 A loaded "-=/x" or "@Group=/x" row: its command is never used. */
export function menuProblem(v: MenuView): string | null {
  return isStructural(v.label) && v.command.trim() ? 'A line or a submenu takes no command.' : null;
}

/** "Shoutout=/shoutout $(nick)" → label + command (split at the first "="). */
export function menuView(r: CommandRow): MenuView {
  const [label, command] = splitMenu(r.text);
  return { enabled: r.enabled, label, command, raw: r.text };
}

/** Unchanged: the old text. Else `label=command`, or just the label; a # label joins with a space. */
export function menuRow(v: MenuView): CommandRow {
  const [label, command] = splitMenu(v.raw);
  if (v.label === label && v.command === command) return { enabled: v.enabled, text: v.raw };
  if (isNote(v.label)) return { enabled: v.enabled, text: v.command ? `${v.label} ${v.command}` : v.label };
  const l = v.label.replaceAll('=', '');
  // V110-F90 A line ("-") or a submenu ("@Group") takes no command: kept, it became an item named "-" or "Group".
  if (isStructural(l)) return { enabled: v.enabled, text: l };
  return { enabled: v.enabled, text: v.command ? `${l}=${v.command}` : l };
}

/** What the line check gets: an off row is an empty line, so errors keep their row numbers. */
export function checkLines(rows: CommandRow[]): string[] {
  return rows.map((r) => (r.enabled ? r.text : ''));
}

/** A "." row under an "@" row that's off: hidden from the menu. Mirrors `Commands::menu_lines`. */
export function hiddenBySubmenu(rows: CommandRow[], i: number): boolean {
  let offSub = false;
  for (let j = 0; j <= i && j < rows.length; j++) {
    const t = rows[j].text.trim();
    if (t === '' || t.startsWith('#')) continue; // blank and note rows don't end a submenu
    if (t.startsWith('@')) offSub = !rows[j].enabled;
    else if (!t.startsWith('.')) offSub = false;
    else if (j === i) return offSub;
  }
  return false;
}
