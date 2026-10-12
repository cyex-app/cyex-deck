// How a line stands out: highlights (2.6), the app it came from (C26), effects (1.42).

import type { IconName } from '../../components/icons';
import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { Client } from '../../ipc/bindings/Client';

export const CLIENTS: Record<Client, { icon: IconName; label: string }> = {
  web: { icon: 'monitor', label: 'Sent from the website' },
  android: { icon: 'smartphone', label: 'Sent from the Android app' },
  ios: { icon: 'smartphone', label: 'Sent from the iPhone app' },
  mobile: { icon: 'smartphone', label: 'Sent from Twitch’s phone app' },
  odd: { icon: 'alert', label: 'Sent from an app that isn’t Twitch’s' },
};

/** A line's highlight class and color. Notices keep their own look unless a color is set. */
export function highlightStyle(m: ChatMessage): { cls: string; vars: string } {
  const h = m.highlight;
  if (!h) return { cls: '', vars: '' };
  if (h.color) return { cls: 'hl', vars: `--hl:${h.color}` };
  if (m.notice) return { cls: '', vars: '' };
  return { cls: `hl hl-${h.kind}`, vars: '' };
}

/** 1.42 Twitch's looping background for Cosmic Abyss. */
export const COSMIC = 'https://d3aqoihi2n8ty8.cloudfront.net/power-ups/cosmic-abyss-tall.mp4';

const EFFECTS = ['simmer', 'rainbow-eclipse', 'cosmic-abyss'] as const;
export type Effect = (typeof EFFECTS)[number];

/** Unknown effects show as a plain line, like twitch.tv. */
export function effectOf(m: ChatMessage): Effect | null {
  const e = m.effect;
  return e && (EFFECTS as readonly string[]).includes(e) ? (e as Effect) : null;
}
