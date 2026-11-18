// V111-59 Chatterino's {element.copytext}: what copying the right-clicked piece of a line gives.
// A word of text, an emoji, a channel name or a cheer; nothing when the click was on no piece.

/** The word of `text` around index `at` (space-separated, as Chatterino's words), or ''. */
export function wordAt(text: string, at: number): string {
  const space = (c: string | undefined) => c === undefined || /\s/.test(c);
  let i = Math.max(0, Math.min(at, text.length));
  // On the space after a word (the end of a text node, a click past the last letter): that word.
  if (space(text[i]) && !space(text[i - 1])) i -= 1;
  if (space(text[i])) return '';
  let from = i;
  let to = i;
  while (!space(text[from - 1])) from -= 1;
  while (!space(text[to])) to += 1;
  return text.slice(from, to);
}

/** The copy text of what `e` right-clicked in a chat line, or null. */
export function copyTextAt(e: MouseEvent): string | null {
  const target = e.target as HTMLElement | null;
  const emoji = target?.closest<HTMLElement>('[data-emoji]');
  if (emoji) return emoji.dataset.emoji || null;
  const piece = target?.closest<HTMLElement>('[data-channel], .cheer');
  if (piece) return piece.textContent?.trim() || null;
  const line = target?.closest('[data-id]');
  const range = document.caretRangeFromPoint?.(e.clientX, e.clientY);
  const node = range?.startContainer;
  if (!line || !node || node.nodeType !== Node.TEXT_NODE || !line.contains(node)) return null;
  return wordAt(node.textContent ?? '', range.startOffset) || null;
}
