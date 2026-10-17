import type { Panel } from '../../ipc/bindings/Panel';

/** V106-21/24 A panel draws something: a title, text, or an image that loaded. */
export function shows(p: Panel, broken: ReadonlySet<string>): boolean {
  return Boolean(p.title) || p.body.length > 0 || (p.image !== null && !broken.has(p.image));
}

/** V106-22 Panels with a link get the hand and a focus ring. */
export function linked(p: Panel): boolean {
  return p.link !== null;
}

/** V106-22 What a screen reader says for a linked panel. */
export function linkLabel(p: Panel): string {
  let host = '';
  try {
    host = p.link ? new URL(p.link).hostname.replace(/^www\./, '') : '';
  } catch {
    host = '';
  }
  const opens = host ? `opens ${host}` : 'opens a link';
  return p.title ? `${p.title}, ${opens}` : `${opens[0].toUpperCase()}${opens.slice(1)}`;
}
