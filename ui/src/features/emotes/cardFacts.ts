// 1.41 What the emote card lists, in reading order: where it's from, who made it, who
// added it, when. Links only where the card can follow them (in-app channel, 7tv.app).
import type { EmoteCard } from '../../ipc/bindings/EmoteCard';
import type { Provider } from '../../ipc/bindings/Provider';

export type Fact =
  /** `tip`: the full date behind a short one. */
  | { label: string; value: string; link?: undefined; tip?: string }
  /** Opens the channel's tab in the app. */
  | { label: string; value: string; link: 'channel'; login: string }
  /** Opens a page in the browser. */
  | { label: string; value: string; link: 'web'; url: string; tip: string };

export interface Chip {
  text: string;
  warn?: boolean;
}

/** The hostname without `www.`, for "Opens 7tv.app". */
function host(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}

export function cardFacts(info: EmoteCard, provider: Provider, added: string | null): Fact[] {
  const out: Fact[] = [];
  if (info.channel) {
    out.push(
      info.channelLogin
        ? { label: 'Channel', value: info.channel, link: 'channel', login: info.channelLogin }
        : { label: 'Channel', value: info.channel },
    );
  }
  if (info.owner) {
    const label = provider === 'twitch' ? 'Artist' : 'By';
    const at = info.ownerUrl ? host(info.ownerUrl) : '';
    out.push(
      info.ownerUrl && at
        ? { label, value: info.owner, link: 'web', url: info.ownerUrl, tip: `Opens ${at}` }
        : { label, value: info.owner },
    );
  }
  if (info.addedBy) out.push({ label: 'Added by', value: info.addedBy });
  if (added) out.push({ label: provider === 'ffz' ? 'Uploaded' : 'Added', value: added, tip: info.addedAt ?? undefined });
  return out;
}

/** The chips after the provider's: kind, state, gigantified, zero-width, unlisted. */
export function cardChips(info: EmoteCard | null, giant: boolean): Chip[] {
  const out: Chip[] = [];
  if (info?.kind) out.push({ text: info.kind });
  if (info?.state) out.push({ text: info.state, warn: true });
  if (giant) out.push({ text: 'Gigantified' });
  if (info?.zeroWidth) out.push({ text: 'Zero-width' });
  if (info?.unlisted) out.push({ text: 'Unlisted', warn: true });
  return out;
}
