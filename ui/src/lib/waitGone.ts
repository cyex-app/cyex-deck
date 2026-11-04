/**
 * V108-F59a Resolves true once `gone()` holds, false if it still does not after `limit` ms.
 * Polls on animation frames so an async close (confirm dialog, out transition) is waited out.
 * `now` and `frame` are injectable for tests.
 */
export function waitGone(
  gone: () => boolean,
  limit = 1500,
  now: () => number = () => performance.now(),
  frame: (cb: () => void) => void = (cb) => void requestAnimationFrame(cb),
): Promise<boolean> {
  const t0 = now();
  return new Promise((resolve) => {
    const step = () => {
      if (gone()) return resolve(true);
      if (now() - t0 > limit) return resolve(false);
      frame(step);
    };
    step();
  });
}
