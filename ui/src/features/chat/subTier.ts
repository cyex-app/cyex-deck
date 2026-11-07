// V109-F74 The tier inside a subscriber badge's version: 0-999 tier 1, 2000+ tier 2, 3000+ tier 3.

/** 2 or 3 for a Tier 2 / Tier 3 subscriber badge version; null for tier 1 or anything unreadable. */
export function subTier(version: string | undefined): 2 | 3 | null {
  if (!version || !/^\d+$/.test(version)) return null;
  const n = Number(version);
  if (n >= 3000 && n < 4000) return 3;
  if (n >= 2000 && n < 3000) return 2;
  return null;
}

/** "7 months", "Tier 2 · 7 months"; the founder badge has no tier. */
export function subHint(set: string, version: string | undefined, info: string | undefined): string | undefined {
  const tier = set === 'subscriber' ? subTier(version) : null;
  const months = info ? `${info} ${info === '1' ? 'month' : 'months'}` : undefined;
  if (!tier) return months;
  return months ? `Tier ${tier} · ${months}` : `Tier ${tier}`;
}
