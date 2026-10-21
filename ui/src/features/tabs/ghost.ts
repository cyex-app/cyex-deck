// V107-R46 Where the dragged name goes: right of and below the pointer, flipped to the
// other side where the window ends there, and always kept inside it.

export interface Point {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

const DX = 12;
const DY = 10;
const EDGE = 4;

/** One axis: after the pointer when it fits, else before it, then clamped to the window. */
function axis(at: number, gap: number, len: number, span: number): number {
  const after = at + gap;
  const v = after + len <= span - EDGE ? after : at - gap - len;
  return Math.max(EDGE, Math.min(v, span - EDGE - len));
}

export function ghostAt(p: Point, size: Size, view: Size): { left: number; top: number } {
  return { left: axis(p.x, DX, size.width, view.width), top: axis(p.y, DY, size.height, view.height) };
}
