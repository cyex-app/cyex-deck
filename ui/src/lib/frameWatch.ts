// V110-F02 Tells Rust the main window still draws: about once a second, one animation frame
// and a ping. A window that stopped painting runs no frames, so its pings stop and Rust's
// watchdog nudges it (windows::paint). Only one frame per second is asked for, so a page
// with nothing moving still idles between them. Nothing goes while the page is hidden,
// except one `false` when it hides, so Rust knows the quiet is on purpose.

export const PING_EVERY_MS = 1000;

/** Starts the pings; `ping(visible)` sends one. Returns the stop. */
export function startFrameWatch(ping: (visible: boolean) => Promise<void>): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let frame = 0;
  const send = (visible: boolean) => void ping(visible).catch(() => {});
  const next = () => {
    timer = setTimeout(() => {
      timer = undefined;
      frame = requestAnimationFrame(() => {
        frame = 0;
        send(true);
        next();
      });
    }, PING_EVERY_MS);
  };
  const stop = () => {
    if (timer !== undefined) clearTimeout(timer);
    if (frame) cancelAnimationFrame(frame);
    timer = undefined;
    frame = 0;
  };
  const vis = () => {
    stop();
    // Shown again: the next frame says so. Not at once, or a nudge (which hides and shows
    // the webview) would count as a frame.
    if (document.visibilityState === 'visible') {
      next();
    } else {
      send(false);
    }
  };
  document.addEventListener('visibilitychange', vis);
  vis();
  return () => {
    stop();
    document.removeEventListener('visibilitychange', vis);
  };
}
