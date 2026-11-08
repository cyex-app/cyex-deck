import assert from 'node:assert/strict';
import { test } from 'node:test';

import { raidTabsToClose } from '../chat/raidBanner.ts';
import { dockRaid, raidTabs } from './raidTabs.ts';

const store = new Map<string, string>();
Object.assign(globalThis, {
  localStorage: {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
  },
});

test('V110-F18 docking a raid pop-out drops its mark, so the tab is not closed with the window', () => {
  raidTabs.add('t1');
  raidTabs.add('t2');
  const seen = new Set<string>();
  raidTabsToClose(raidTabs.list(), new Set(['t1', 't2']), seen);
  dockRaid('t1');
  assert.deepEqual(raidTabs.list(), ['t2']);
  assert.deepEqual(raidTabsToClose(raidTabs.list(), new Set(['t2']), seen), []);
  assert.deepEqual(raidTabsToClose(raidTabs.list(), new Set(), seen), ['t2'], 'a window that closes undocked still takes its tab');
});

test('V110-F18 leftover marks are dropped, the rest stay', () => {
  raidTabs.add('t3');
  raidTabs.add('t4');
  raidTabs.keep(['t4']);
  assert.deepEqual(raidTabs.list(), ['t4']);
  raidTabs.keep([]);
});
