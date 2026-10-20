// V107-R19 Circle crop for the profile picture: the image covers a square view, can be
// dragged and zoomed, and never leaves a gap. Pure math; the dialog draws with it.

export interface Crop {
  /** Image top-left in view pixels (≤ 0). */
  x: number;
  y: number;
  /** View pixels per image pixel. */
  scale: number;
}

export const ZOOM_MAX = 4;
/** Twitch's profile picture size. */
export const OUT_SIZE = 240;

/** Smallest scale that still covers the view. */
export function coverScale(w: number, h: number, view: number): number {
  return view / Math.min(w, h);
}

function clampAxis(pos: number, len: number, view: number): number {
  return Math.min(0, Math.max(view - len, pos));
}

export function clamp(c: Crop, w: number, h: number, view: number): Crop {
  return { scale: c.scale, x: clampAxis(c.x, w * c.scale, view), y: clampAxis(c.y, h * c.scale, view) };
}

/** Centered, covering. */
export function initial(w: number, h: number, view: number): Crop {
  const scale = coverScale(w, h, view);
  return { scale, x: (view - w * scale) / 2, y: (view - h * scale) / 2 };
}

export function drag(c: Crop, dx: number, dy: number, w: number, h: number, view: number): Crop {
  return clamp({ ...c, x: c.x + dx, y: c.y + dy }, w, h, view);
}

/** Zoom (1 = cover, up to ZOOM_MAX) around the view's center. */
export function zoomTo(c: Crop, zoom: number, w: number, h: number, view: number): Crop {
  const base = coverScale(w, h, view);
  const scale = base * Math.min(ZOOM_MAX, Math.max(1, zoom));
  const mid = view / 2;
  const ix = (mid - c.x) / c.scale;
  const iy = (mid - c.y) / c.scale;
  return clamp({ scale, x: mid - ix * scale, y: mid - iy * scale }, w, h, view);
}

/** The square of the image (image pixels) that ends up in the picture. */
export function source(c: Crop, view: number): { sx: number; sy: number; size: number } {
  return { sx: 0 - c.x / c.scale, sy: 0 - c.y / c.scale, size: view / c.scale };
}
