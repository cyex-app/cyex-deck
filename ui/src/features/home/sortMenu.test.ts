import assert from 'node:assert/strict';
import { test } from 'node:test';

import { menuProblems } from '../../components/menuItems.ts';
import { sortEntries } from './sortMenu.ts';

test('V110-N4 Home sort is one flat list: live sorts, a line, offline sorts', () => {
  const set: unknown[] = [];
  const list = sortEntries({ liveSort: 'uptime', offlineSort: 'name' }, (p) => set.push(p));
  assert.deepEqual(
    list.map((e) => (e === 'separator' ? '-' : `${e.label}|${e.hint}|${e.checked ? 'x' : ''}`)),
    ['Most viewers|Live|', 'Live the longest|Live|x', 'Name|Live|', '-', 'Live most recently|Offline|', 'Name|Offline|x'],
  );
  assert.deepEqual(menuProblems(list), []);
  const last = list[5];
  if (last !== 'separator') last.run?.();
  assert.deepEqual(set, [{ offlineSort: 'name' }]);
});
