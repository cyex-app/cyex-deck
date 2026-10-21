import assert from 'node:assert/strict';
import { test } from 'node:test';

import { navHistory, type NavTarget } from './navHistory.svelte.ts';

test('V107-R33 the mouse buttons walk back and forward, skipping gone places', () => {
  let here = 'a';
  const gone = new Set<string>();
  const nav = navHistory<string>({ same: (a, b) => a === b, here: () => here, ok: (p) => !gone.has(p), go: (p) => go(p) });
  const go = (p: string) => {
    nav.left(here); // ignored while walking
    here = p;
  };
  const visit = (p: string) => {
    nav.left(here);
    here = p;
  };
  visit('b');
  visit('c');
  gone.add('b');
  const handlers = new Map<string, (e: MouseEvent) => void>();
  const target: NavTarget = {
    addEventListener: (t, f) => handlers.set(t, f),
    removeEventListener: (t) => handlers.delete(t),
  };
  const stop = nav.listen(target);
  let prevented = 0;
  const press = (button: number) => handlers.get('mouseup')?.({ button, preventDefault: () => prevented++ } as unknown as MouseEvent);
  press(3);
  assert.equal(here, 'a');
  press(4);
  assert.equal(here, 'c');
  press(0);
  assert.equal(prevented, 2);
  stop();
  assert.equal(handlers.size, 0);
});
