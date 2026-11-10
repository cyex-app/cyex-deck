// V110-F86 What the message box remembers: the Up recall, a draft per chat, text limits.

import { CappedMap } from '../../lib/capped.ts';

/** The recall position while the box still shows the recalled line, else -1: typing something
 *  else ends the recall, so Up moves the caret instead of replacing the text. */
export function syncRecall(recall: number, text: string, sent: readonly string[]): number {
  return recall >= 0 && sent[recall] === text ? recall : -1;
}

/** Whether Up recalls a sent line: the box is empty or still shows a recalled one. */
export function upRecalls(text: string, recall: number, sent: readonly string[]): boolean {
  return sent.length > 0 && (!text || syncRecall(recall, text, sent) >= 0);
}

/** V110-A-idea Whether a paste is longer than the box takes: the text kept, minus the selection it replaces, plus the paste. */
export function pasteCut(kept: number, selected: number, pasted: string, max: number): boolean {
  return kept - selected + pasted.split('\r\n').join('\n').length > max;
}

/** A cut at maxlength can leave half of an emoji at the end. */
export function dropLoneHigh(text: string): string {
  const c = text.charCodeAt(text.length - 1);
  return c >= 0xd800 && c <= 0xdbff ? text.slice(0, -1) : text;
}

export interface Draft {
  text: string;
  sent: string[];
}

const CHATS = 50;
const LINES = 50;
const drafts = new CappedMap<string, Draft>(CHATS);

/** The draft and sent lines of a chat (`account:login`). V110-A-idea The draft text (not the sent lines) also
 *  comes back after a restart, from this viewer's storage. */
export function loadDraft(key: string): Draft {
  restore();
  const d = drafts.get(key);
  return d ? { text: d.text, sent: [...d.sent] } : { text: '', sent: [] };
}

export function saveDraft(key: string, text: string, sent: readonly string[]): void {
  restore();
  if (!text && !sent.length) drafts.delete(key);
  else drafts.set(key, { text, sent: sent.slice(0, LINES) });
  later();
}

// V110-A-idea Drafts across restarts: small, written a moment after the last keystroke.
export const DRAFTS_KEY = 'cyex.chatDrafts';
/** Longest draft kept, in characters (the box allows 1.000). */
export const DRAFT_CHARS = 1000;
export const DRAFT_WAIT_MS = 800;

type Store = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>;
let store: Store | null | undefined;
let restored = false;
let timer: ReturnType<typeof setTimeout> | undefined;

function disk(): Store | null {
  if (store !== undefined) return store;
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

/** Reads the saved drafts once; a draft typed since stays. */
function restore(): void {
  if (restored) return;
  restored = true;
  try {
    const v: unknown = JSON.parse(disk()?.getItem(DRAFTS_KEY) ?? '[]');
    if (!Array.isArray(v)) return;
    for (const e of v.slice(-CHATS)) {
      if (!Array.isArray(e) || typeof e[0] !== 'string' || typeof e[1] !== 'string' || !e[1]) continue;
      if (!drafts.has(e[0])) drafts.set(e[0], { text: dropLoneHigh(e[1].slice(0, DRAFT_CHARS)), sent: [] });
    }
  } catch {
    // Unreadable storage or a broken value: no drafts from before.
  }
}

/** Writes the drafts that have text; none left removes the key. */
export function persistDrafts(): void {
  clearTimeout(timer);
  timer = undefined;
  try {
    const list = [...drafts].filter(([, d]) => d.text).map(([k, d]) => [k, d.text.slice(0, DRAFT_CHARS)]);
    if (list.length) disk()?.setItem(DRAFTS_KEY, JSON.stringify(list.slice(-CHATS)));
    else disk()?.removeItem(DRAFTS_KEY);
  } catch {
    // Blocked or full storage: drafts last until the window closes.
  }
}

function later(): void {
  clearTimeout(timer);
  timer = setTimeout(persistDrafts, DRAFT_WAIT_MS);
}

if (typeof addEventListener === 'function') addEventListener('pagehide', () => timer !== undefined && persistDrafts());

/** Test hook: another storage (or none) and a fresh start. */
export function resetDrafts(next?: Store | null): void {
  clearTimeout(timer);
  timer = undefined;
  drafts.clear();
  store = next;
  restored = false;
}

export function draftKey(account: string | null, login: string): string {
  return `${account ?? ''}:${login.toLowerCase()}`;
}

/** Test hook. */
export function clearDrafts(): void {
  drafts.clear();
}
