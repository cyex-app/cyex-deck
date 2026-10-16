/** Sequence guard: only the newest request may apply its result. */
export function latest() {
  let n = 0;
  return {
    /** Starts a request. The returned check is true only until the next begin()/cancel(). */
    begin(): () => boolean {
      const id = ++n;
      return () => id === n;
    },
    /** Drops everything in flight. */
    cancel() {
      n++;
    },
  };
}
