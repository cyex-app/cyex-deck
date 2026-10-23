import assert from 'node:assert/strict';
import { test } from 'node:test';

// .svelte.ts modules: outside the compiler, $state is the plain value.
const g = globalThis as unknown as { $state: { <T>(v: T): T; raw: <T>(v: T) => T } };
g.$state = Object.assign(<T>(v: T) => v, { raw: <T>(v: T) => v });
const { ClearUndo } = await import('./clearUndo.svelte.ts');

function feed(start: string[]) {
  let rows = [...start];
  const committed: string[][] = [];
  const u = new ClearUndo(
    () => rows,
    async (keys) => void committed.push(keys),
    (gone) => (rows = rows.filter((r) => !gone.has(r))),
  );
  return { u, committed, rows: () => rows, add: (r: string) => rows.push(r) };
}

test('V107-MF Clear hides at once; Undo brings the rows back and clears nothing', () => {
  const f = feed(['a', 'b']);
  f.u.clear();
  assert.deepEqual([f.u.shows('a'), f.u.shows('b')], [false, false]);
  f.u.undo();
  assert.equal(f.u.hidden, null);
  f.u.flush();
  assert.deepEqual(f.committed, []);
  assert.deepEqual(f.rows(), ['a', 'b']);
});

test('V107-MF flush drops only the hidden rows; one that came in meanwhile stays', () => {
  const f = feed(['a', 'b']);
  f.u.clear();
  f.add('c');
  assert.equal(f.u.shows('c'), true);
  f.u.flush();
  assert.deepEqual(f.committed, [['a', 'b']]);
  assert.deepEqual(f.rows(), ['c']);
  f.u.flush();
  assert.equal(f.committed.length, 1, 'a second flush sends nothing');
});

test('V107-MF Clear on an empty feed does nothing; a second Clear commits the first', () => {
  const f = feed([]);
  f.u.clear();
  assert.equal(f.u.hidden, null);
  const g2 = feed(['a']);
  g2.u.clear();
  g2.add('b');
  g2.u.clear();
  assert.deepEqual(g2.committed, [['a']]);
  assert.equal(g2.u.shows('b'), false);
  g2.u.flush();
  assert.deepEqual(g2.committed, [['a'], ['b']]);
});
