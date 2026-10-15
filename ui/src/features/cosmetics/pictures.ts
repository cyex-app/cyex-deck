// V1051-03 Which badge picture to load: the smallest scale that covers the cell.

/** 7TV badges are 18 px at 1x. */
const BASE = 18;

function pick(list: readonly string[], want: number): string {
  for (let n = want; n <= list.length; n++) if (list[n - 1]) return list[n - 1];
  for (let n = Math.min(want, list.length) - 1; n >= 1; n--) if (list[n - 1]) return list[n - 1];
  return '';
}

export function badgePicture(
  images: readonly string[],
  stills: readonly string[] | null,
  cssPx: number,
  dpr: number,
  still: boolean,
): string {
  const want = Math.min(4, Math.max(1, Math.ceil((cssPx * dpr) / BASE)));
  if (still && stills) {
    const s = pick(stills, want);
    if (s) return s;
  }
  return pick(images, want);
}
