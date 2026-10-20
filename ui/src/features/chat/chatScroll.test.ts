import assert from 'node:assert/strict';
import { test } from 'node:test';

// A .svelte.ts module: outside the compiler, $state is the plain value.
const g = globalThis as unknown as { $state: { <T>(v?: T): T | undefined; raw: <T>(v: T) => T } };
g.$state = Object.assign(<T>(v?: T) => v, { raw: <T>(v: T) => v });
const { ChatScroll } = await import('./chatScroll.svelte.ts');
const { SHRINK_MS, STEP, WINDOW } = await import('./lineWindow.ts');

/** Timers that run only when told. */
class FakeClock {
  t = 0;
  private timers = new Map<number, { at: number; fn: () => void }>();
  private next = 1;
  now = () => this.t;
  setTimeout = (fn: () => void, ms: number) => {
    const id = this.next++;
    this.timers.set(id, { at: this.t + ms, fn });
    return id;
  };
  clearTimeout = (id: unknown) => void this.timers.delete(id as number);
  advance(ms: number) {
    const end = this.t + ms;
    for (;;) {
      const due = [...this.timers].filter(([, x]) => x.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
      if (!due) break;
      this.timers.delete(due[0]);
      this.t = due[1].at;
      due[1].fn();
    }
    this.t = end;
  }
}

type Msg = { id: string; ts: number };
const make = (from: number, n: number): Msg[] => Array.from({ length: n }, (_, i) => ({ id: `m${from + i}`, ts: (from + i) * 10 }));

function setup(n = 400, opts = { pauseOnHover: true, pauseStillTenths: 0 }) {
  const msgs = make(0, n);
  const clock = new FakeClock();
  let shrinks = 0;
  const s = new ChatScroll({ lines: () => msgs, messages: () => msgs, onshrink: () => shrinks++ }, opts, clock);
  const add = (k: number) => msgs.push(...make(msgs.length, k));
  return { s, msgs, clock, add, shrinks: () => shrinks };
}

// List metrics: a 600 px list with 20 px lines.
const at = (top: number, height = 8000, client = 600) => ({ top, height, client });
const bottom = (height = 8000) => at(height - 600, height);

test('following at the bottom draws the newest WINDOW lines', () => {
  const { s, add } = setup();
  assert.equal(s.following, true);
  assert.equal(s.paused, false);
  assert.equal(s.drawn.length, WINDOW);
  add(500);
  assert.equal(s.drawn.at(-1)!.id, 'm899');
  assert.equal(s.drawn.length, WINDOW);
  assert.equal(s.fresh, 0);
  assert.equal(s.scrolled(bottom()), null);
  assert.equal(s.pill, false);
});

test('scrolling up while lines arrive holds the window and counts them', () => {
  const { s, add } = setup();
  s.scrolled(bottom());
  s.leave();
  assert.equal(s.following, false);
  const first = s.drawn[0].id;
  add(40);
  assert.equal(s.drawn[0].id, first);
  assert.equal(s.drawn.length, WINDOW + 40);
  assert.equal(s.fresh, 40);
  s.measured(at(3000));
  assert.equal(s.pill, true);
});

test('a scroll that moved itself while following snaps back down', () => {
  const { s } = setup();
  s.scrolled(bottom());
  assert.equal(s.scrolled(at(7000)), 'bottom');
  assert.equal(s.following, true);
});

test('back at the bottom, the window shrinks after SHRINK_MS', () => {
  const { s, clock, add, shrinks } = setup();
  s.scrolled(bottom());
  s.leave();
  add(40);
  assert.equal(s.drawn.length, WINDOW + 40);
  assert.equal(s.scrolled(bottom(8800)), null);
  assert.equal(s.following, true);
  assert.equal(s.fresh, 0);
  clock.advance(SHRINK_MS - 1);
  assert.equal(s.drawn.length, WINDOW + 40);
  assert.equal(shrinks(), 0);
  clock.advance(1);
  assert.equal(s.drawn.length, WINDOW);
  assert.equal(shrinks(), 1);
});

test('leaving the bottom again before SHRINK_MS keeps the window', () => {
  const { s, clock, add, shrinks } = setup();
  s.leave();
  add(10);
  s.scrolled(bottom());
  clock.advance(1000);
  s.leave();
  clock.advance(1000);
  assert.equal(s.drawn.length, WINDOW + 10);
  assert.equal(shrinks(), 0);
});

test('near the top draws older lines, STEP at a time, once per draw', () => {
  const { s } = setup(1000);
  s.scrolled(bottom());
  s.leave();
  assert.equal(s.scrolled(at(100)), 'older');
  assert.equal(s.drawn.length, WINDOW + STEP);
  // Still drawing: no second step.
  assert.equal(s.scrolled(at(90)), null);
  s.grew();
  assert.equal(s.scrolled(at(80)), 'older');
  assert.equal(s.drawn.length, WINDOW + 2 * STEP);
});

test('a jump to an undrawn line draws it, then stops following', () => {
  const { s } = setup(1000);
  assert.equal(s.drawAround('m900'), false);
  assert.equal(s.drawAround('m120'), true);
  assert.equal(s.drawn[0].id, 'm70');
  assert.equal(s.drawAround('m120'), false);
  assert.equal(s.drawAround('nope'), false);
  s.jumped();
  assert.equal(s.following, false);
  assert.equal(s.drawn[0].id, 'm70');
});

test('a jump while following, then back down, shrinks again', () => {
  const { s, clock } = setup(1000);
  s.drawAround('m120');
  s.jumped();
  s.scrolled(bottom());
  clock.advance(SHRINK_MS);
  assert.equal(s.drawn.length, WINDOW);
});

test('paused while a menu from the list is open', () => {
  const { s, add } = setup();
  s.menu(true);
  assert.equal(s.paused, true);
  assert.equal(s.following, true);
  const first = s.drawn[0].id;
  add(5);
  assert.equal(s.drawn[0].id, first);
  assert.equal(s.fresh, 5);
  s.measured(at(3000));
  assert.equal(s.pill, true);
  // A scroll the list didn't ask for stays put while paused.
  assert.equal(s.scrolled(at(3000)), null);
  s.menu(false);
  assert.equal(s.paused, false);
  assert.equal(s.fresh, 0);
});

test('the scrollbar drag leaves the bottom; a snap does not', () => {
  const { s } = setup();
  s.scrolled(bottom());
  // A programmatic move up (lines above sized in) while following: snap back.
  assert.equal(s.scrolled(at(7000)), 'bottom');
  assert.equal(s.following, true);
  s.drag(true);
  assert.equal(s.scrolled(at(6900)), null);
  assert.equal(s.following, false);
  // Dragging back down to the bottom follows again.
  assert.equal(s.scrolled(bottom()), null);
  assert.equal(s.following, true);
  s.drag(false);
});

test('the first placement ignores the hover pause (v1.07 t1)', () => {
  const { s } = setup();
  // A tab opens under a resting mouse: paused from the start.
  s.enter();
  assert.equal(s.paused, true);
  assert.equal(s.sticks, true);
  s.measured(at(0));
  assert.equal(s.pill, false);
  s.landed();
  s.measured(bottom());
  assert.equal(s.sticks, false);
  assert.equal(s.pill, false);
});

test('pause on hover until the mouse rests, and the pill', () => {
  const { s, clock, add } = setup(400, { pauseOnHover: true, pauseStillTenths: 20 });
  s.landed();
  s.enter();
  assert.equal(s.paused, true);
  assert.equal(s.fresh, 0);
  s.measured(at(3000));
  assert.equal(s.pill, false);
  add(3);
  assert.equal(s.fresh, 3);
  assert.equal(s.pill, true);
  clock.advance(1999);
  s.move();
  clock.advance(1999);
  assert.equal(s.paused, true);
  clock.advance(1);
  assert.equal(s.paused, false);
  assert.equal(s.sticks, true);
  assert.equal(s.fresh, 0);
  s.exit();
  assert.equal(s.paused, false);
});

test('hover does not pause when the setting is off', () => {
  const { s } = setup(400, { pauseOnHover: false, pauseStillTenths: 0 });
  s.enter();
  assert.equal(s.paused, false);
  s.configure({ pauseOnHover: true, pauseStillTenths: 0 });
  assert.equal(s.paused, true);
});

test('the pause key, and the pill clears both pauses', () => {
  const { s, add } = setup();
  s.landed();
  s.key(true);
  assert.equal(s.paused, true);
  add(2);
  assert.equal(s.fresh, 2);
  s.key(false);
  assert.equal(s.paused, false);
  s.enter();
  s.key(true);
  s.toNewest();
  assert.equal(s.paused, false);
  assert.equal(s.following, true);
});

test('an empty chat that is left counts nothing', () => {
  const msgs: Msg[] = [];
  const s = new ChatScroll({ lines: () => msgs, messages: () => msgs, onshrink: () => {} }, { pauseOnHover: true, pauseStillTenths: 0 }, new FakeClock());
  s.leave();
  msgs.push(...make(0, 3));
  assert.equal(s.fresh, 0);
});

test('a slide never outlasts the gap since the last lines, and carries its end', () => {
  const { s, clock } = setup();
  assert.equal(s.glide(24, 600, false, false) > 0, true);
  clock.advance(100);
  assert.equal(s.glide(24, 600, false, false), 100);
  clock.advance(40);
  // Grew while that slide played: only what's left of it.
  assert.equal(s.glide(10, 600, true, true), 60);
  assert.equal(s.glide(10, 600, true, false), 0);
  assert.equal(s.glide(0.5, 600, false, false), 0);
});
