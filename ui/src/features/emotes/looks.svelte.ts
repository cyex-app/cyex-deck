// Provider badges, 7TV paints and BTTV name effects by user (1.37-1.39). Names ask as
// they're drawn, in batches; changes arrive from Rust, so a paint that shows up after a
// message still lands on it. Each paint becomes one CSS class.

import { SvelteMap } from 'svelte/reactivity';

import type { BadgeDef } from '../../ipc/bindings/BadgeDef';
import type { Look } from '../../ipc/bindings/Look';
import type { Looks } from '../../ipc/bindings/Looks';
import type { PaintStyle } from '../../ipc/bindings/PaintStyle';
import { emotesIpc } from '../../ipc/emotes';

const BATCH_MS = 60;
/** Paint ids are 7TV ULIDs; anything else never becomes a class name. */
const SAFE_ID = /^[0-9A-Za-z]{1,40}$/;

class LooksStore {
  private users = new SvelteMap<string, Look | null>();
  private badges = new SvelteMap<string, BadgeDef>();
  private queue = new Set<string>();
  private timer: ReturnType<typeof setTimeout> | undefined;
  private paints = new Set<string>();
  /** V101-36 Paint names by id, for the tip on a painted name. */
  private paintNames = new Map<string, string>();
  private sheet: HTMLStyleElement | null = null;
  private started = false;

  /** What a user wears; asks Rust the first time. */
  look(userId: string): Look | null {
    const l = this.users.get(userId);
    if (l === undefined) this.request(userId);
    return l ?? null;
  }

  badge(provider: string, id: string): BadgeDef | undefined {
    return this.badges.get(`${provider}:${id}`);
  }

  /** CSS class for a paint, or `null` if it can't be drawn. */
  paintClass(id: string | null): string | null {
    return id && this.paints.has(id) ? `paint-${id}` : null;
  }

  /** V101-36 What 7TV calls a paint. */
  paintName(id: string | null): string | null {
    return (id && this.paintNames.get(id)) || null;
  }

  start(): () => void {
    if (this.started) return () => {};
    this.started = true;
    const off = emotesIpc.onLooks((l) => this.apply(l));
    return () => void off.then((f) => f());
  }

  private request(id: string) {
    if (!id) return;
    this.queue.add(id);
    this.timer ??= setTimeout(() => this.flush(), BATCH_MS);
  }

  private async flush() {
    this.timer = undefined;
    const ids = [...this.queue];
    this.queue.clear();
    // Marked as asked right away so nothing asks twice.
    for (const id of ids) if (!this.users.has(id)) this.users.set(id, null);
    try {
      this.apply(await emotesIpc.looks(ids));
    } catch {
      // Lookups are best effort; names just stay plain.
    }
  }

  private apply(l: Looks) {
    for (const p of l.paints) this.addPaint(p);
    for (const b of l.badges) this.badges.set(`${b.provider}:${b.id}`, b);
    for (const [id, look] of Object.entries(l.users)) this.users.set(id, look ?? null);
  }

  private addPaint(p: PaintStyle) {
    if (p.name.trim()) this.paintNames.set(p.id, p.name.trim());
    if (this.paints.has(p.id) || !SAFE_ID.test(p.id)) return;
    this.sheet ??= document.head.appendChild(document.createElement('style'));
    const rule = (sel: string, css: string) => `${sel}{${css.replace(/[{}<]/g, '')}}`;
    this.sheet.append(
      rule(`.paint-${p.id}`, p.css) + (p.still ? rule(`:root.still .paint-${p.id}`, p.still) : '') + '\n',
    );
    this.paints.add(p.id);
  }
}

export const looks = new LooksStore();
