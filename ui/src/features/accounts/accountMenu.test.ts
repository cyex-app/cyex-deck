import assert from 'node:assert/strict';
import { test } from 'node:test';

import { type MenuEntry, menuProblems } from '../../components/menuItems.ts';
import { accountRows } from './accountMenu.ts';

const noop = () => {};
const rows = (walletLock: 'helix' | null, connect: (() => void) | null) =>
  accountRows({ walletLock, connect, ask: noop, onmanage: noop, onsubs: noop, bits: noop, wallet: noop, ondashboard: noop, oncosmetics: noop, oncyex: noop });
const names = (l: MenuEntry[]): string[] => l.map((e) => (e === 'separator' ? '-' : e.items ? `${e.label}>${names(e.items).join(',')}` : e.lock ? `${e.label}!` : e.label));

test('V111-N1 account menu: Twitch pages sit in one submenu', () => {
  assert.deepEqual(names(rows(null, null)), [
    'Manage accounts',
    '-',
    'Creator dashboard',
    'Twitch>Subscriptions,Get Bits,Payment methods',
    '-',
    '7TV paint and badge',
    'cyex cosmetics',
  ]);
  assert.match(names(rows(null, noop))[3], /Payment methods,Connect Twitch pages$/);
  assert.deepEqual(menuProblems(rows(null, noop)), ['label "cyex cosmetics"']);
});

test('V111-N1 account menu: locked, the Twitch row carries the lock and no greyed rows show', () => {
  const list = rows('helix', null);
  assert.deepEqual(names(list).slice(2, 4), ['Creator dashboard', 'Twitch!']);
  const t = list[3];
  assert.ok(t !== 'separator' && t.lock === 'helix' && !t.items && t.unlock);
});
