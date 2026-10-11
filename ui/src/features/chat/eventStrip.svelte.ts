// V103-04 What's hidden into the "N Events" button, per channel (rules in eventStrip.ts).

import { SvelteMap } from 'svelte/reactivity';
import { untrack } from 'svelte';

import { close, EMPTY_STRIP, hide, reopen, settle, type ChatEvent, type StripState } from './eventStrip';

class EventStrip {
  private readonly byChannel = new SvelteMap<string, StripState>();

  of(channel: string): StripState {
    return this.byChannel.get(channel) ?? EMPTY_STRIP;
  }

  /** Called whenever the pin, poll or prediction changes. */
  settle(channel: string, events: readonly ChatEvent[]): void {
    untrack(() => this.put(channel, settle(this.of(channel), events)));
  }

  hide(channel: string, e: ChatEvent): void {
    this.put(channel, hide(this.of(channel), e));
  }

  close(channel: string, key: string): void {
    this.put(channel, close(this.of(channel), key));
  }

  reopen(channel: string): void {
    this.put(channel, reopen(this.of(channel)));
  }

  private put(channel: string, next: StripState): void {
    if (next !== this.of(channel)) this.byChannel.set(channel, next);
  }
}

export const eventStrip = new EventStrip();
