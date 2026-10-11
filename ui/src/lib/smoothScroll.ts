// V103-11 Smooth scrolling in every scrollable area, Chatterino's way (`scrollMath.ts`).
// A root effect runs it while the setting is on, so a change applies at once. While on,
// one wheel listener on the window takes notched wheel turns over anything that can scroll
// that way. Touchpads, Ctrl zoom, sideways turns and parts that use the wheel themselves
// (they call `preventDefault`) stay with the browser. Off, nothing listens. One frame loop
// per window, and only while something glides. WebView2's own animation is always off
// (`--disable-smooth-scrolling`), so keys and the scrollbar jump, as in Chatterino.

import { aim, carry, frame, type Glide, moves, NOTCH_PX, notches } from './scrollMath';

/** `last`: where the view was put, to tell when something else moved it. */
type Run = { glide: Glide; last: number };

const runs = new Map<Element, Run>();
let loop = 0;

/** Turns that come quickly, with the pointer still, stay with the area the first one
 * scrolled, as in the browser: a list that reaches its end doesn't hand the rest of a fast
 * spin to the page around it. */
const LATCH_MS = 500;
const LATCH_PX = 10;
/** Weak, so a closed chat isn't kept alive by the last turn over it. */
type Latch = { el: WeakRef<Element>; x: number; y: number; until: number };
let latch: Latch | null = null;

const SCROLLS = new Set(['auto', 'scroll', 'overlay']);

export function startSmoothScroll(): () => void {
  addEventListener('wheel', onwheel, { passive: false });
  addEventListener('pointerdown', stopAll, { capture: true, passive: true });
  return () => {
    removeEventListener('wheel', onwheel);
    removeEventListener('pointerdown', stopAll, { capture: true });
    stopAll();
    latch = null;
  };
}

/** Code that jumps an area (to the bottom, to a line) ends its glide first. */
export function stopScroll(el: Element): void {
  runs.delete(el);
}

/** A press (the scrollbar, a button) leaves every glide where it is. */
function stopAll() {
  if (!runs.size) return;
  runs.clear();
  cancelAnimationFrame(loop);
  loop = 0;
}

function onwheel(e: WheelEvent) {
  if (e.defaultPrevented || e.ctrlKey || e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
  const n = notches((e as WheelEvent & { wheelDeltaY?: number }).wheelDeltaY, e.deltaY, e.deltaMode);
  const target = e.target;
  if (!n || !(target instanceof Element) || target.matches('input[type="number"]:focus')) return;
  const by = n * NOTCH_PX;
  const now = performance.now();
  const held = latched(e, target, now);
  const el = held?.el.deref() ?? scroller(target, by);
  if (!el) return;
  e.preventDefault();
  latch = held
    ? { ...held, until: now + LATCH_MS }
    : { el: new WeakRef(el), x: e.clientX, y: e.clientY, until: now + LATCH_MS };
  glide(el, by, now);
}

function latched(e: WheelEvent, target: Element, now: number): Latch | null {
  if (!latch || now >= latch.until) return null;
  if (Math.abs(e.clientX - latch.x) > LATCH_PX || Math.abs(e.clientY - latch.y) > LATCH_PX) return null;
  return latch.el.deref()?.contains(target) ? latch : null;
}

/** The nearest area from the pointer up that can still scroll `by` px. */
function scroller(from: Element, by: number): Element | null {
  for (let el: Element | null = from; el; el = el.parentElement) {
    const max = el.scrollHeight - el.clientHeight;
    if (max < 1) continue;
    const style = getComputedStyle(el);
    if (!SCROLLS.has(style.overflowY)) continue;
    const run = runs.get(el);
    if (moves(run ? run.glide.to : el.scrollTop, by, max)) return el;
    if (style.overscrollBehaviorY !== 'auto') return null;
  }
  return null;
}

function glide(el: Element, by: number, now: number) {
  const max = el.scrollHeight - el.clientHeight;
  const run = runs.get(el);
  const g = aim(run ? carry(run.glide, el.scrollTop - run.last, max) : null, el.scrollTop, by, max, now);
  if (!g) return;
  runs.set(el, { glide: g, last: el.scrollTop });
  loop ||= requestAnimationFrame(tick);
}

function tick() {
  loop = 0;
  const now = performance.now();
  for (const [el, run] of runs) {
    if (!el.isConnected) {
      runs.delete(el);
      continue;
    }
    const max = el.scrollHeight - el.clientHeight;
    run.glide = carry(run.glide, el.scrollTop - run.last, max);
    const f = frame(run.glide, now, max);
    el.scrollTop = f.top;
    run.last = el.scrollTop;
    if (f.done) runs.delete(el);
  }
  if (runs.size) loop = requestAnimationFrame(tick);
}
