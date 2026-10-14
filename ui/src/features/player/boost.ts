// V105-10 Volume past 100 %: what the element does and what the gain adds on top.

/** 500 %, Rust's MAX_BOOST. */
export const MAX_BOOST = 5;

/** The highest volume the slider allows. */
export function maxVolume(boost: boolean): number {
  return boost ? MAX_BOOST : 1;
}

export function clampVolume(v: number, boost: boolean): number {
  return Math.min(maxVolume(boost), Math.max(0, v));
}

/** The element plays up to 100 %; the gain takes over above it. */
export function split(volume: number): { element: number; gain: number } {
  return volume <= 1 ? { element: volume, gain: 1 } : { element: 1, gain: volume };
}
