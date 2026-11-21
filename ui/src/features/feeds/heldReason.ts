// V108-N15 The label on a held message. The backend sends `automod:<category>[:<level>]`,
// `blocked_term[:<term>\u001f<term>…]` (V112-28: every matched term, `crates/moderation/src/held.rs`)
// or `blocked_link`; a bare word (the older PubSub path) is a category.

/** Terms shown before the rest becomes an ellipsis, and the longest one shown whole. */
const SHOWN_TERMS = 3;
const TERM_CHARS = 32;

/** "sexuality_sex_or_gender" -> "Sexuality sex or gender". */
function words(raw: string): string {
  const t = raw.replace(/_/g, ' ').trim();
  return t ? t[0].toUpperCase() + t.slice(1) : '';
}

export function heldLabel(reason: string): string {
  if (reason === 'blocked_link') return 'Blocked link';
  if (reason === 'blocked_term' || reason === 'blocked term') return 'Blocked term';
  if (reason.startsWith('blocked_term:')) {
    const terms = termsOf(reason.slice('blocked_term:'.length));
    if (!terms.length) return 'Blocked term';
    const shown = terms.slice(0, SHOWN_TERMS).join(', ');
    return `Blocked term · ${shown}${terms.length > SHOWN_TERMS ? ', …' : ''}`;
  }
  if (reason.startsWith('automod:')) {
    const [category, level] = reason.slice('automod:'.length).split(':');
    const name = words(category ?? '') || 'AutoMod';
    return level ? `${name} · ${level}` : name;
  }
  return words(reason) || 'AutoMod';
}

/** V112-28 The wire's terms, blanks dropped, long ones cut. */
function termsOf(raw: string): string[] {
  return raw
    .split('\u001f')
    .map((t) => t.trim())
    .filter(Boolean)
    .map((t) => ([...t].length > TERM_CHARS ? `${[...t].slice(0, TERM_CHARS - 1).join('')}…` : t));
}
