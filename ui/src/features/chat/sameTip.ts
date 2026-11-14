// V111-02 Whether two elements show the same tip (the same link, emoji, time or tip text), so a
// chat line drawn again under a resting pointer keeps its open tip.

type Tipped = { dataset: Record<string, string | undefined> };

const KEYS = ['link', 'emoji', 'stamp', 'tip'] as const;

export function sameTip(a: Tipped, b: Tipped): boolean {
  return KEYS.some((k) => a.dataset[k] !== undefined) && KEYS.every((k) => a.dataset[k] === b.dataset[k]);
}
