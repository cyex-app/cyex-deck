// V106-20 The story progress bar runs on the compositor: one Web Animation on `transform`,
// never a per-frame state write. Pictures end the animation themselves; videos drive it.
// Web Animations, not CSS keyframes: Reduce motion zeroes CSS animation durations (tokens.css).

const KEYS: Keyframe[] = [{ transform: 'scaleX(0)' }, { transform: 'scaleX(1)' }];

/** Seek drift (ms) that makes a video's bar re-sync. */
export const DRIFT_MS = 250;

/** A picture: fills in `ms`, then `onend`. Returns the stop function. */
export function pictureBar(el: Element, ms: number, onend: () => void): () => void {
  const a = el.animate(KEYS, { duration: ms, easing: 'linear', fill: 'forwards' });
  a.onfinish = onend;
  return () => {
    a.onfinish = null;
    a.cancel();
  };
}

const VIDEO_EVENTS = ['loadedmetadata', 'durationchange', 'playing', 'pause', 'waiting', 'seeked', 'ratechange'] as const;

/** A video: the bar follows its clock (play, pause, waiting, seek, rate). Returns the stop function. */
export function videoBar(el: Element, v: HTMLVideoElement): () => void {
  let a: Animation | null = null;
  const sync = () => {
    if (!Number.isFinite(v.duration) || v.duration <= 0) return;
    const duration = v.duration * 1000;
    if (a) a.effect?.updateTiming({ duration });
    else a = el.animate(KEYS, { duration, easing: 'linear', fill: 'forwards' });
    a.playbackRate = v.playbackRate;
    a.currentTime = v.currentTime * 1000;
    if (v.paused || v.readyState < 3) a.pause();
    else a.play();
  };
  const drift = () => {
    if (a && Math.abs(Number(a.currentTime) - v.currentTime * 1000) > DRIFT_MS) sync();
  };
  for (const e of VIDEO_EVENTS) v.addEventListener(e, sync);
  v.addEventListener('timeupdate', drift);
  sync();
  return () => {
    for (const e of VIDEO_EVENTS) v.removeEventListener(e, sync);
    v.removeEventListener('timeupdate', drift);
    a?.cancel();
  };
}
