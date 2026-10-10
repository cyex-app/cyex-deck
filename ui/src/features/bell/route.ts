// Where a Twitch link goes (1.26): a channel opens as a tab, any other twitch.tv page in
// the app's Twitch window, everything else in the browser. The user shouldn't need
// twitch.tv in a browser.

export type Route = { kind: 'channel'; login: string } | { kind: 'twitch'; url: string } | { kind: 'web'; url: string };

/** Paths on twitch.tv that are pages, not channels. */
const PAGES = new Set([
  'directory',
  'settings',
  'wallet',
  'payments',
  'subscriptions',
  'subs',
  'products',
  'unsubscribe',
  'drops',
  'inventory',
  'videos',
  'search',
  'downloads',
  'jobs',
  'p',
  'prime',
  'turbo',
  'u',
  'moderator',
  'popout',
  'login',
  'signup',
  'messages',
  'friends',
  'following',
  'activate',
  'bits',
  'giftcard',
]);

const LOGIN = /^[a-z0-9_]{1,25}$/;
const CHANNEL_PAGES = new Set(['', 'about', 'videos', 'clips', 'schedule']);

export function route(url: string): Route {
  let u: URL;
  try {
    u = new URL(url, 'https://www.twitch.tv');
  } catch {
    return { kind: 'web', url };
  }
  const host = u.hostname.toLowerCase();
  const twitch = host === 'twitch.tv' || host.endsWith('.twitch.tv');
  if (!twitch || u.protocol !== 'https:') return { kind: 'web', url: u.href };
  const parts = u.pathname.split('/').filter(Boolean);
  const first = (parts[0] ?? '').toLowerCase();
  if ((host === 'www.twitch.tv' || host === 'twitch.tv') && LOGIN.test(first) && !PAGES.has(first)) {
    if (CHANNEL_PAGES.has((parts[1] ?? '').toLowerCase()) && parts.length <= 2) return { kind: 'channel', login: first };
  }
  return { kind: 'twitch', url: u.href };
}
