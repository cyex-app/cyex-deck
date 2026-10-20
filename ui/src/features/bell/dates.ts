// V107-R9 Twitch writes dates into bell texts its own way ("November 10, 2026 7:59:00 am UTC").
// They're rewritten to the app's format in local time: "2026/11/10 08:59" (3.1, 3.2).
// A time without a zone is UTC: Twitch never knows the viewer's zone. A date alone stays that day.

import { formatDate, formatTime } from '../../lib/format.ts';

const MONTHS = ['jan', 'feb', 'mar', 'apr', 'may', 'jun', 'jul', 'aug', 'sep', 'oct', 'nov', 'dec'];
const MONTH =
  '(jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|june?|july?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\\.?';
const TIME = '(?:,?\\s+(?:at\\s+)?(\\d{1,2}):(\\d{2})(?::(\\d{2}))?(?:\\s*([ap])\\.?m\\.?)?)?';
const ZONE = '(?:\\s*(utc|gmt|z|[+-]\\d{2}:?\\d{2})(?![a-z]))?';

// "November 10, 2026", "Nov 10th 2026 at 7:59 PM UTC"
const MONTH_FIRST = new RegExp(`\\b${MONTH}\\s+(\\d{1,2})(?:st|nd|rd|th)?,?\\s+(\\d{4})\\b${TIME}${ZONE}`, 'gi');
// "10 November 2026, 19:59 UTC"
const DAY_FIRST = new RegExp(`\\b(\\d{1,2})(?:st|nd|rd|th)?\\s+${MONTH},?\\s+(\\d{4})\\b${TIME}${ZONE}`, 'gi');
// "2026-11-10T07:59:00Z", "2026-11-10 07:59 UTC", "2026-11-10"
const ISO = new RegExp(
  `\\b(\\d{4})-(\\d{2})-(\\d{2})(?:[T ](\\d{2}):(\\d{2})(?::(\\d{2})(?:\\.\\d+)?)?)?${ZONE.replace('\\s*', '\\s?')}`,
  'gi',
);

interface Parts {
  y: string;
  mo: number;
  d: string;
  h?: string;
  mi?: string;
  s?: string;
  ap?: string;
  zone?: string;
}

function render(p: Parts, raw: string): string {
  const y = Number(p.y);
  const d = Number(p.d);
  if (p.mo < 0 || p.mo > 11 || d < 1 || d > 31) return raw;
  if (p.h === undefined) return formatDate(new Date(y, p.mo, d));
  let h = Number(p.h);
  if (p.ap) {
    if (h < 1 || h > 12) return raw;
    h = (h % 12) + (p.ap.toLowerCase() === 'p' ? 12 : 0);
  }
  if (h > 23 || Number(p.mi) > 59) return raw;
  let ms = Date.UTC(y, p.mo, d, h, Number(p.mi), Number(p.s ?? 0));
  const off = p.zone?.match(/^([+-])(\d{2}):?(\d{2})$/);
  if (off) ms -= (off[1] === '-' ? -1 : 1) * (Number(off[2]) * 60 + Number(off[3])) * 60_000;
  const at = new Date(ms);
  return `${formatDate(at)} ${formatTime(at)}`;
}

const month = (name: string) => MONTHS.indexOf(name.slice(0, 3).toLowerCase());

/** Every date in `text` in the app's format; text without dates comes back as is. */
export function localDates(text: string): string {
  if (!/\d{4}/.test(text)) return text;
  return text
    .replace(MONTH_FIRST, (raw, mo, d, y, h, mi, s, ap, zone) =>
      render({ y, mo: month(mo), d, h, mi, s, ap, zone }, raw),
    )
    .replace(DAY_FIRST, (raw, d, mo, y, h, mi, s, ap, zone) =>
      render({ y, mo: month(mo), d, h, mi, s, ap, zone }, raw),
    )
    .replace(ISO, (raw, y, mo, d, h, mi, s, zone) => render({ y, mo: Number(mo) - 1, d, h, mi, s, zone }, raw));
}
