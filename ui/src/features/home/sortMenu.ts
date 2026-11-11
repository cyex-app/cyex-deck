// V111-N1 The Sort menu on Home: one flat list, the live sorts and then the offline ones. The
// row hint says which part a row sorts ("Name" is in both).

import type { MenuEntry } from '../../components/menuItems';
import type { LiveSort } from '../../ipc/bindings/LiveSort';
import type { OfflineSort } from '../../ipc/bindings/OfflineSort';

const LIVE: [LiveSort, string][] = [
  ['viewers', 'Most viewers'],
  ['uptime', 'Live the longest'],
  ['name', 'Name'],
];
const OFFLINE: [OfflineSort, string][] = [
  ['lastLive', 'Live most recently'],
  ['name', 'Name'],
];

export function sortEntries(
  picked: { liveSort: LiveSort; offlineSort: OfflineSort },
  set: (patch: { liveSort: LiveSort } | { offlineSort: OfflineSort }) => void,
): MenuEntry[] {
  return [
    ...LIVE.map(([v, label]): MenuEntry => ({ label, hint: 'Live', checked: picked.liveSort === v, run: () => set({ liveSort: v }) })),
    'separator',
    ...OFFLINE.map(([v, label]): MenuEntry => ({ label, hint: 'Offline', checked: picked.offlineSort === v, run: () => set({ offlineSort: v }) })),
  ];
}
