import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Combo } from '../../ipc/cosmetics';
import { current, pairTitle, problems, problemText, withNote } from './combos.ts';

const mk = (id: string, paint: string | null, badge: string | null): Combo => ({
  id,
  name: id,
  paint: paint ? { id: paint, name: `P ${paint}` } : null,
  badge: badge ? { id: badge, name: `B ${badge}` } : null,
});
const list = [mk('a', 'p1', 'b1'), mk('b', 'p1', null), mk('c', 'p1', null), mk('d', null, null)];

test('current is the first match, nulls included', () => {
  assert.equal(current(list, 'p1', 'b1')?.id, 'a');
  assert.equal(current(list, 'p1', null)?.id, 'b');
  assert.equal(current(list, null, null)?.id, 'd');
  assert.equal(current(list, 'p2', null), null);
});

test('problems and their text', () => {
  const have = (k: 'paint' | 'badge', id: string) => (k === 'paint' && id === 'p1' ? 'notOwned' : id === 'b1' ? 'needsSub' : 'usable');
  const p = problems(list[0], have);
  assert.deepEqual(p.map((x) => x.kind), ['paint', 'badge']);
  assert.equal(problemText(p), 'You don’t own P p1 any more. B b1 is for 7TV subscribers.');
  assert.deepEqual(problems(list[3], have), []);
  assert.equal(problemText([]), '');
});

test('pairTitle names a missing part', () => {
  assert.equal(pairTitle(list[1]), 'P p1 + No badge');
  assert.equal(pairTitle(list[3]), 'No paint + No badge');
});

test('a worn combo whose paint is gone gets the broken note', () => {
  const p = problems(mk('a', 'p1', null), () => 'notOwned');
  assert.equal(withNote('Wearing a', p), 'Wearing a. You don’t own P p1 any more.');
  assert.equal(withNote('Wearing a', []), 'Wearing a');
  assert.equal(withNote('Wearing a without its paint: gone.', p), 'Wearing a without its paint: gone.');
});
