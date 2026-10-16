// C8 Where a video with object-fit: contain is really drawn inside its element.

export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** The picture's rect inside a box of `boxW` x `boxH`, for a video of `vw` x `vh`. An unknown
 * size (0) gives the whole box. */
export function containRect(boxW: number, boxH: number, vw: number, vh: number): Rect {
  if (!(vw > 0 && vh > 0 && boxW > 0 && boxH > 0)) return { left: 0, top: 0, width: boxW, height: boxH };
  const k = Math.min(boxW / vw, boxH / vh);
  const width = vw * k;
  const height = vh * k;
  return { left: (boxW - width) / 2, top: (boxH - height) / 2, width, height };
}
