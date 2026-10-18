// V106-31/32 7TV's rule for an emote's name in a set (research-phase-5 §5), checked before any request.

export const ALIAS_RE = /^[\p{L}\p{M}\p{Nd}\p{Pc}\p{Join_Control}\-():!+|.'?><&\p{Emoji_Presentation}*$#]{1,100}$/u;

export type AliasCheck = { ok: true; name: string } | { ok: false; message: string };

export function checkAlias(text: string): AliasCheck {
  if (text === '') return { ok: false, message: 'Give it a name.' };
  if ([...text].length > 100) return { ok: false, message: '100 characters at most.' };
  if (/\s/.test(text)) return { ok: false, message: 'No spaces.' };
  if (!ALIAS_RE.test(text)) return { ok: false, message: '7TV doesn’t allow a character in this name.' };
  return { ok: true, name: text };
}

/** c-12 The whole verdict for `text`, read at call time: an error line, '' for "not ok, say
 *  nothing" (Rust's check), null when fine. Submit uses it so a key in the same tick as typing
 *  can't act on the last text's answer. */
export async function judgeAlias(
  text: string,
  check?: (name: string) => Promise<string | null>,
): Promise<string | null> {
  const r = checkAlias(text);
  if (!r.ok) return r.message;
  return check ? await check(r.name) : null;
}
