// V1051-01 Decoded audio as Rust plays it: stereo, interleaved, a limited length.

export interface Decoded {
  numberOfChannels: number;
  length: number;
  getChannelData(channel: number): Float32Array;
}

/** Left and right samples one after the other, at most `maxFrames`. Mono plays on both sides. */
export function interleave(buf: Decoded, maxFrames: number): Float32Array {
  const frames = Math.min(buf.length, maxFrames);
  const left = buf.getChannelData(0);
  const right = buf.numberOfChannels > 1 ? buf.getChannelData(1) : left;
  const out = new Float32Array(frames * 2);
  for (let i = 0; i < frames; i++) {
    out[i * 2] = left[i];
    out[i * 2 + 1] = right[i];
  }
  return out;
}
