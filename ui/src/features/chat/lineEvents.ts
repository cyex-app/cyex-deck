// What a chat line's parts do, shared by every list of lines (chat, the usercard's logs):
// emote card and emote menu, @mentions, #channels, links and uploads, and the tips over
// badges, links and emoji. The lists delegate their clicks, keys and hovers here.

import { hideTip, previewTip, retitleTip, showTip } from '../../components/tooltip.svelte';
import type { MenuEntry } from '../../components/menu.svelte';
import { popOut } from '../../app/panels';
import type { LinkTarget } from '../../ipc/bindings/LinkTarget';
import { chatIpc } from '../../ipc/chat';
import { systemIpc } from '../../ipc/system';
import { copy } from '../../lib/clipboard';
import { errorText } from '../../lib/errors';
import { settings } from '../../lib/settings.svelte';
import { streamer } from '../../lib/streamer.svelte';
import { toast } from '../../lib/toast.svelte';
import { emojiName } from '../emotes/emoji';
import type { CardTarget } from '../emotes/emotes';
import { cancelEmoteMenu, emoteEditMenu } from '../emotes/edit/menu';
import { removeHere } from '../emotes/edit/run';
import { hidden } from '../emotes/hidden.svelte';
import { flipUnlisted, unlistedBlurred } from '../emotes/unlisted';
import { roomRoute } from './roomRoute';

/** The parts `openPart` handles. */
export const PARTS = '[data-user],[data-channel],[data-link],[data-upload],[data-emote]';

export interface LineHost {
  /** The chat the lines are from. */
  channel: string;
  /** The account reading it, `null` when anonymous. */
  account: string | null;
  /** Who sent the line `el` is in. */
  senderOf: (el: HTMLElement) => string | undefined;
  openCard: (t: CardTarget) => void;
  openMenu: (e: MouseEvent, items: MenuEntry[]) => void;
  /** Twitch clips and VODs open in the app (1.19, 1.20, P7) instead of the browser. */
  onlink?: (target: LinkTarget) => void;
}

/** 1.41 The emote under `el`, placed for its card. */
export function emoteAt(el: HTMLElement, host: LineHost): CardTarget {
  const d = JSON.parse(el.dataset.emote!) as Omit<CardTarget, 'rect'>;
  const sender = host.senderOf(el);
  return { ...d, ...(sender && { sender }), rect: el.getBoundingClientRect() };
}

/** Runs a click on a line's emote, link, upload, #channel or @name. False for anything else. */
export function openPart(e: MouseEvent, el: HTMLElement, host: LineHost): boolean {
  const d = el.dataset;
  if (d.emote) {
    hideTip();
    host.openCard(emoteAt(el, host));
  } else if (d.link) {
    e.preventDefault();
    if (d.target && host.onlink) host.onlink(JSON.parse(d.target) as LinkTarget);
    else systemIpc.openChatLink(d.link).catch((err) => toast.error(errorText(err)));
  } else if (d.upload) {
    e.preventDefault();
    systemIpc.openUpload(d.upload).catch((err) => toast.error(errorText(err)));
  } else if (d.channel) {
    // V102-05 As this chat's account, or anonymously like it.
    hideTip();
    void popOut(roomRoute(d.channel, host.account), `${d.channel} chat`);
  } else if (d.user) {
    chatIpc.openUsercard(host.channel, d.user).catch((err) => toast.error(errorText(err)));
  } else return false;
  return true;
}

/** Enter on a focused emote, name or #channel (they're spans, not buttons) clicks it. */
export function enterClicks(e: KeyboardEvent): void {
  if (e.key !== 'Enter' || e.repeat || e.defaultPrevented) return;
  const el = e.target as HTMLElement;
  if (el.tagName === 'A' || el.tagName === 'BUTTON' || !el.matches?.('[role="button"],[role="link"]')) return;
  e.preventDefault();
  el.click();
}

/** E4 Right-click an emote: card, copy, hide, and 7TV edits (K11 V106-31/32). */
export async function emoteMenu(e: MouseEvent, el: HTMLElement, host: LineHost): Promise<void> {
  e.preventDefault();
  const t = emoteAt(el, host);
  const { emote } = t;
  const { account, channel } = host;
  hideTip();
  const unlisted = el.dataset.unlisted !== undefined;
  const edits = await emoteEditMenu(account, channel, emote, {
    add: (target) => host.openCard({ ...t, edit: { kind: 'add', target } }),
    rename: (here) => host.openCard({ ...t, edit: { kind: 'rename', here } }),
    remove: (h) => void removeHere(account!, channel, emote.id, h),
  });
  // Null: the user moved on or a newer right-click came in.
  if (!edits) return;
  host.openMenu(e, [
    { label: 'Emote card', icon: 'smile', run: () => host.openCard(t) },
    { label: 'Copy name', icon: 'copy', run: () => void copy(emote.name, 'Emote name') },
    'separator',
    {
      label: `Hide ${emote.name}`,
      icon: 'eyeOff',
      run: () => hidden.hide({ source: emote.provider, id: emote.id, name: emote.name }),
    },
    // V106-31/32 Add, rename, remove; none of it without a 7TV sign-in and an editable channel (Q2, Q6).
    ...edits,
    // V102-01 One switch for every unlisted 7TV emote, in every chat.
    ...(unlisted
      ? ['separator' as const, { label: 'Blur unlisted emotes', icon: 'eye' as const, checked: unlistedBlurred(), run: flipUnlisted }]
      : []),
  ]);
}

/** A right-click away from an emote drops a 7TV menu still being read. */
export const notEmote = cancelEmoteMenu;

/** 1.52 Picture widths, Chatterino's sizes. */
const THUMBS = { off: 0, small: 100, medium: 200, large: 300 } as const;
const TIPPED = '[data-tip],[data-link],[data-emoji]';

/** Tips for badges, links (X2, 1.52) and emoji (V102-07) inside `root`. c-4 A line that drops
 *  off, scrolls away or goes with the tab gives no mouseout, so its tip goes with it. */
export function lineTips(root: () => HTMLElement | undefined) {
  let tipEl: HTMLElement | null = null;
  const gone = new MutationObserver(() => tipEl && !tipEl.isConnected && drop());

  function hold(el: HTMLElement) {
    tipEl = el;
    const r = root();
    if (r) gone.observe(r, { childList: true, subtree: true });
  }

  function drop() {
    if (!tipEl) return;
    tipEl = null;
    gone.disconnect();
    hideTip();
  }

  function emojiTip(el: HTMLElement) {
    emojiName(el.dataset.emoji!)
      .then((name) => name && el.matches(':hover') && showTip(el, { label: name }))
      .catch(() => {});
  }

  /** The real domain right away, the final one once redirects are followed; a preview
   *  with it when previews are on. */
  function linkTip(el: HTMLElement) {
    const url = el.dataset.link!;
    const host = el.dataset.host!;
    showTip(el, { label: `Opens ${host}` });
    chatIpc
      .resolveLink(url)
      .then((info) => {
        if (!info || tipEl !== el) return;
        if (info.link) {
          const finalHost = new URL(info.link).hostname.replace(/^www\./, '');
          if (finalHost !== host) retitleTip(`Opens ${host}, ends up on ${finalHost}`);
        }
        const c = settings.v.chat;
        if (c.linkPreviews && !streamer.hides('hideLinkPreviews') && (info.lines.length || info.thumbnail)) {
          previewTip(el, { lines: info.lines, thumbnail: info.thumbnail ?? '', thumbWidth: THUMBS[c.linkThumbnails] });
        }
      })
      .catch(() => {});
  }

  return {
    onmouseover(e: MouseEvent) {
      const el = (e.target as HTMLElement).closest<HTMLElement>(TIPPED);
      if (!el) return;
      hold(el);
      if (el.dataset.link) return linkTip(el);
      if (el.dataset.emoji) return emojiTip(el);
      showTip(el, { label: el.dataset.tip ?? '', hint: el.dataset.hint, image: el.dataset.tipImg });
    },
    onmouseout(e: MouseEvent) {
      const from = (e.target as HTMLElement).closest(TIPPED);
      if (from && !from.contains(e.relatedTarget as Node)) drop();
    },
    drop,
  };
}
