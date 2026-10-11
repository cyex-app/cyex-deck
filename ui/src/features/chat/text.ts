import type { Fragment } from '../../ipc/bindings/Fragment';

/** A message body as plain text, e.g. for reply previews. */
export function plainText(body: Fragment[]): string {
  return body
    .map((f) => {
      switch (f.t) {
        case 'text':
        case 'mention':
        case 'link':
          return f.text;
        case 'emote':
          return f.name;
        case 'cheer':
          return `${f.prefix}${f.amount}`;
        case 'gif':
          return 'GIF';
        case 'span':
          return f.text;
        case 'image':
          return f.alt;
        case 'break':
          return '\n';
      }
    })
    .join('');
}
