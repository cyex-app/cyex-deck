import type { LinkHealth } from '../../../ipc/bindings/LinkHealth';

export interface HealthRow {
  label: string;
  count: number;
  when: string;
  why: string;
}

/** "42 s ago", "5 min ago", "2 h ago". */
export function ago(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  if (s < 90) return `${s} s ago`;
  const m = Math.round(s / 60);
  if (m < 90) return `${m} min ago`;
  return `${Math.round(m / 60)} h ago`;
}

export function healthRows(list: LinkHealth[], now: number): HealthRow[] {
  return list.map((l) => ({
    label: l.label,
    count: l.count,
    when: l.lastAt ? ago(now - l.lastAt) : '–',
    why: l.lastError || '–',
  }));
}
