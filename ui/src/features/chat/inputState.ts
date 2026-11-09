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

/** The draft and sent lines of a chat (`account:login`); not kept across restarts. */
export function loadDraft(key: string): Draft {
  const d = drafts.get(key);
  return d ? { text: d.text, sent: [...d.sent] } : { text: '', sent: [] };
}

export function saveDraft(key: string, text: string, sent: readonly string[]): void {
  if (!text && !sent.length) drafts.delete(key);
  else drafts.set(key, { text, sent: sent.slice(0, LINES) });
}

export function draftKey(account: string | null, login: string): string {
  return `${account ?? ''}:${login.toLowerCase()}`;
}

/** Test hook. */
export function clearDrafts(): void {
  drafts.clear();
}
