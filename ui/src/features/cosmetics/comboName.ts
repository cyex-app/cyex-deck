// V1051-04 Mirror of Rust's `clean_name` (crates/settings/src/schema/combos). The shared
// fixture keeps the two in step. `\p{Alphabetic}` and `\p{White_Space}` are what Rust's
// `is_alphanumeric` and `split_whitespace` use (marks in Hindi and Thai names count).

export const MAX_NAME = 32;
const WORDS = ['list', 'save', 'delete', 'rename'];

export type NameResult =
  | { ok: true; name: string }
  | { ok: false; error: 'empty' | 'tooLong' | 'chars' | 'word'; message: string };

const fail = (error: 'empty' | 'tooLong' | 'chars' | 'word', message: string): NameResult => ({ ok: false, error, message });

/** Rust's taken-name error quotes the name, which can fill the small card. */
export function shortError(message: string): string {
  return /^You have a combo called .* already\.$/su.test(message) ? 'That name’s taken.' : message;
}

export function cleanName(raw: string): NameResult {
  const name = raw.split(/\p{White_Space}+/u).filter(Boolean).join(' ');
  if (!name) return fail('empty', 'Give it a name.');
  if ([...name].length > MAX_NAME) return fail('tooLong', `Names can be up to ${MAX_NAME} characters.`);
  if (!/^[\p{Alphabetic}\p{N} _’-]+$/u.test(name)) return fail('chars', 'Names can have letters, numbers, spaces, ’, - and _.');
  if (WORDS.includes(name.replace(/[A-Z]/g, (c) => c.toLowerCase()))) {
    return fail('word', `“${name}” is part of /combo. Pick another name.`);
  }
  return { ok: true, name };
}
