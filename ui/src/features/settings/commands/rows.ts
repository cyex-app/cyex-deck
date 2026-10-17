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

/** Split at the first space; the rest is kept as is. */
function splitSpace(text: string): [string, string] {
  const t = text.trimStart();
  const at = t.search(/\s/);
  if (at < 0) return [t, ''];
  return [t.slice(0, at), t.slice(at + 1)];
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

/** Unchanged name and sends: the old text. Else name (spaces removed) + " " + sends, or just the name. */
export function commandRow(v: CommandView): CommandRow {
  const [name, sends] = splitSpace(v.raw);
  if (v.name === name && v.sends === sends) return { enabled: v.enabled, text: v.raw };
  const n = v.name.replace(/\s+/g, '');
  return { enabled: v.enabled, text: v.sends ? (n ? `${n} ${v.sends}` : v.sends) : n };
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
    if (t.startsWith('@')) offSub = !rows[j].enabled;
    else if (!t.startsWith('.')) offSub = false;
    else if (j === i) return offSub;
  }
  return false;
}
