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
export const SAFE_ID = /^[0-9A-Za-z]{1,40}$/;

/** V1051-03 Both CSS rules of a paint. `.still` also matches inside any still container. */
export function paintRules(p: PaintStyle): string {
  const rule = (sel: string, css: string) => `${sel}{${css.replace(/[{}<]/g, '')}}`;
  return rule(`.paint-${p.id}`, p.css) + (p.still ? rule(`.still .paint-${p.id}`, p.still) : '') + '\n';
}

class LooksStore {
  private users = new SvelteMap<string, Look | null>();
  private badges = new SvelteMap<string, BadgeDef>();
  private queue = new Set<string>();
  private timer: ReturnType<typeof setTimeout> | undefined;
  private paints = new Set<string>();
  /** V1051-03 Bumped when the preview defines a new paint, so a line already drawn picks it up. */
  private defined = $state(0);
  private definedN = 0;
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
    void this.defined;
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
    this.append(this.fresh(l.paints));
    for (const b of l.badges) this.badges.set(`${b.provider}:${b.id}`, b);
    for (const [id, look] of Object.entries(l.users)) this.users.set(id, look ?? null);
  }

  /** V1051-03 Definitions without users (the cosmetics preview's paint and badge). */
  define(paints: PaintStyle[], badges: BadgeDef[]) {
    const css = this.fresh(paints);
    this.append(css);
    if (css) this.defined = ++this.definedN;
    for (const b of badges) this.badges.set(`${b.provider}:${b.id}`, b);
  }

  /** Notes names; returns the rules of paints whose class is new. */
  private fresh(paints: PaintStyle[]): string {
    let css = '';
    for (const p of paints) {
      if (p.name.trim()) this.paintNames.set(p.id, p.name.trim());
      if (this.paints.has(p.id) || !SAFE_ID.test(p.id)) continue;
      this.paints.add(p.id);
      css += paintRules(p);
    }
    return css;
  }

  private append(css: string) {
    if (!css) return;
    this.sheet ??= document.head.appendChild(document.createElement('style'));
    this.sheet.append(css);
  }
}

export const looks = new LooksStore();
