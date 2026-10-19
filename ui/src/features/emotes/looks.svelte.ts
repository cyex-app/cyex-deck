// Provider badges, 7TV paints and BTTV name effects by user (1.37-1.39). Names ask as
// they're drawn, in batches; changes arrive from Rust, so a paint that shows up after a
// message still lands on it. Each paint becomes one CSS class, made the first time a
// name asks for it (L2: pushed looks carry paints of users nobody draws).

import { SvelteMap } from 'svelte/reactivity';

import type { BadgeDef } from '../../ipc/bindings/BadgeDef';
import type { Look } from '../../ipc/bindings/Look';
import type { Looks } from '../../ipc/bindings/Looks';
import type { PaintStyle } from '../../ipc/bindings/PaintStyle';
import { emotesIpc } from '../../ipc/emotes';
import { applyAnswer, applyPushed } from './lookMap';

const BATCH_MS = 60;
/** Paint ids are 7TV ULIDs; anything else never becomes a class name. */
export const SAFE_ID = /^[0-9A-Za-z]{1,40}$/;

/** V1051-03 Both CSS rules of a paint. `.still` also matches inside any still container. */
export function paintRuleList(p: PaintStyle): string[] {
  const rule = (sel: string, css: string) => `${sel}{${css.replace(/[{}<]/g, '')}}`;
  return [rule(`.paint-${p.id}`, p.css), ...(p.still ? [rule(`.still .paint-${p.id}`, p.still)] : [])];
}

export function paintRules(p: PaintStyle): string {
  return paintRuleList(p).join('') + '\n';
}

class LooksStore {
  private users = new SvelteMap<string, Look | null>();
  private badges = new SvelteMap<string, BadgeDef>();
  private queue = new Set<string>();
  private timer: ReturnType<typeof setTimeout> | undefined;
  /** Paints with their rules in the sheet. */
  private paints = new Set<string>();
  /** L2 Paints known but not drawn yet: their rules go in on first use. */
  private pending = new Map<string, PaintStyle>();
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
    if (!id) return null;
    if (!this.paints.has(id)) {
      const p = this.pending.get(id);
      if (!p) return null;
      this.insert([p]);
    }
    return `paint-${id}`;
  }

  /** V101-36 What 7TV calls a paint. */
  paintName(id: string | null): string | null {
    return (id && this.paintNames.get(id)) || null;
  }

  start(): () => void {
    if (this.started) return () => {};
    this.started = true;
    const off = emotesIpc.onLooks((l) => this.apply(l, true));
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
      this.apply(await emotesIpc.looks(ids), false);
    } catch {
      // Lookups are best effort; names just stay plain.
    }
  }

  /** `pushed`: a change from Rust. Only users already drawn here are kept (V107-M4). */
  private apply(l: Looks, pushed: boolean) {
    this.note(l.paints);
    for (const b of l.badges) this.badges.set(`${b.provider}:${b.id}`, b);
    if (pushed) applyPushed(this.users, l.users);
    else applyAnswer(this.users, l.users);
  }

  /** V1051-03 Definitions without users (the cosmetics preview's paint and badge). */
  define(paints: PaintStyle[], badges: BadgeDef[]) {
    this.note(paints);
    if (this.insert(paints)) this.defined = ++this.definedN;
    for (const b of badges) this.badges.set(`${b.provider}:${b.id}`, b);
  }

  /** Notes names and the paints not drawn yet. */
  private note(paints: PaintStyle[]) {
    for (const p of paints) {
      if (p.name.trim()) this.paintNames.set(p.id, p.name.trim());
      if (!this.paints.has(p.id) && SAFE_ID.test(p.id)) this.pending.set(p.id, p);
    }
  }

  /** Puts the rules of new paints in the sheet; true if any went in. insertRule adds one
   *  rule without parsing the whole sheet again (L2: appended text re-parsed all of it). */
  private insert(paints: PaintStyle[]): boolean {
    let any = false;
    for (const p of paints) {
      if (this.paints.has(p.id) || !SAFE_ID.test(p.id)) continue;
      this.paints.add(p.id);
      this.pending.delete(p.id);
      this.sheet ??= document.head.appendChild(document.createElement('style'));
      const sheet = this.sheet.sheet;
      for (const r of paintRuleList(p)) {
        try {
          sheet?.insertRule(r, sheet.cssRules.length);
        } catch {
          // A rule the engine can't read: that name just stays plain.
        }
      }
      any = true;
    }
    return any;
  }
}

export const looks = new LooksStore();
