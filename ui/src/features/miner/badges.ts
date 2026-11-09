// V110-U68 Badge text for the Miner view, pure so it can be tested: the header line,
// "34/60 min", end dates and what is left.

import type { BadgeView } from '../../ipc/bindings/BadgeView';
import type { MinedChannel } from '../../ipc/bindings/MinedChannel';

const pad = (n: number) => String(n).padStart(2, '0');

/** Unix seconds as yyyy/mm/dd; '' when unknown (0). */
export function endDate(secs: number): string {
  if (!(secs > 0)) return '';
  const d = new Date(secs * 1000);
  return `${d.getFullYear()}/${pad(d.getMonth() + 1)}/${pad(d.getDate())}`;
}

/** "34/60 min", numbers grouped with dots. */
export function progress(b: Pick<BadgeView, 'minutesWatched' | 'minutesRequired'>): string {
  const g = (n: number) => String(Math.max(0, Math.round(n))).replace(/\B(?=(\d{3})+(?!\d))/g, '.');
  return `${g(Math.min(b.minutesWatched, b.minutesRequired))}/${g(b.minutesRequired)} min`;
}

/** Minutes still to watch; never negative. */
export function minutesLeft(b: Pick<BadgeView, 'minutesWatched' | 'minutesRequired'>): number {
  return Math.max(0, b.minutesRequired - b.minutesWatched);
}

/** Share of the bar, 0 to 100. */
export function percent(b: Pick<BadgeView, 'minutesWatched' | 'minutesRequired'>): number {
  if (b.minutesRequired <= 0) return 0;
  return Math.min(100, Math.max(0, (b.minutesWatched / b.minutesRequired) * 100));
}

/** Picked and not finished: the badges the miner still wants. */
export function wanted(badges: readonly BadgeView[]): BadgeView[] {
  return badges.filter((b) => b.picked && minutesLeft(b) > 0);
}

export interface LineInput {
  badges: readonly BadgeView[] | undefined;
  focus: string | null | undefined;
  channels: readonly Pick<MinedChannel, 'name' | 'mining' | 'watching'>[];
  /** When the miner looks again, already formatted (24-hour), if known. */
  retry?: string;
}

/** V110-F77 A badge as text, with "badge" said once: "Mock badge" stays, "Arcane" becomes "Arcane badge". */
export function phrase(name: string): string {
  const n = name.trim();
  if (!n) return 'badge';
  return n.toLowerCase().endsWith('badge') ? n : `${n} badge`;
}

/** The header line for badges, or '' when none is wanted. */
export function statusLine({ badges, focus, channels, retry }: LineInput): string {
  const list = badges ?? [];
  if (focus) {
    const b = list.find((x) => x.campaignId === focus);
    if (b) {
      const on = channels.find((c) => c.mining)?.name;
      return `Earning ${phrase(b.name)}${on ? ` on ${on}` : ''} · ${progress(b)}`;
    }
  }
  if (!wanted(list).length) return '';
  if (channels.filter((c) => c.watching).length >= 2) return 'Badge paused: 2 streams playing here.';
  return `Badge waits: nothing live can earn it${retry ? `, looking again ${retry}` : ''}`;
}
