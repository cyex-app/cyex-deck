// V108-N15 The label on a held message. The backend sends `automod:<category>[:<level>]`,
// `blocked_term[:<text>]` or `blocked_link`; a bare word (the older PubSub path) is a category.

/** "sexuality_sex_or_gender" -> "Sexuality sex or gender". */
function words(raw: string): string {
  const t = raw.replace(/_/g, ' ').trim();
  return t ? t[0].toUpperCase() + t.slice(1) : '';
}

export function heldLabel(reason: string): string {
  if (reason === 'blocked_link') return 'Blocked link';
  if (reason === 'blocked_term' || reason === 'blocked term') return 'Blocked term';
  if (reason.startsWith('blocked_term:')) {
    const term = reason.slice('blocked_term:'.length).trim();
    return term ? `Blocked term: ${term}` : 'Blocked term';
  }
  if (reason.startsWith('automod:')) {
    const [category, level] = reason.slice('automod:'.length).split(':');
    const name = words(category ?? '') || 'AutoMod';
    return level ? `${name} · ${level}` : name;
  }
  return words(reason) || 'AutoMod';
}
