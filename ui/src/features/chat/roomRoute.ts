// V102-05 The window for a `#channel` from chat: `room/<login>` read anonymously, or
// `room/<login>/<account id>` as that account. One window per route.

const LOGIN = /^[a-z0-9_]{1,25}$/;
const ID = /^\d+$/;

export function roomRoute(login: string, account: string | null): string {
  const l = login.toLowerCase();
  return account ? `room/${l}/${account}` : `room/${l}`;
}

export function parseRoomRoute(route: string): { login: string; account: string | null } | null {
  const [kind, login = '', account, ...rest] = route.split('/');
  if (kind !== 'room' || !LOGIN.test(login) || rest.length) return null;
  if (account !== undefined && !ID.test(account)) return null;
  return { login, account: account ?? null };
}
