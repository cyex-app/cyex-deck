import assert from 'node:assert/strict';
import { test } from 'node:test';

import { PlaceHistory } from './placeHistory.ts';

const same = (a: string, b: string) => a === b;
const any = () => true;

test('V107-R33 back and forward walk the places shown', () => {
  const h = new PlaceHistory(same);
  h.left('a');
  h.left('b');
  assert.equal(h.goBack('c', any), 'b');
  assert.equal(h.goBack('b', any), 'a');
  assert.equal(h.goBack('a', any), null);
  assert.equal(h.goForward('a', any), 'b');
  assert.equal(h.goForward('b', any), 'c');
  assert.equal(h.goForward('c', any), null);
});

test('V107-R33 going somewhere new clears forward', () => {
  const h = new PlaceHistory(same);
  h.left('a');
  assert.equal(h.goBack('b', any), 'a');
  h.left('a');
  assert.equal(h.goForward('c', any), null);
  assert.equal(h.goBack('c', any), 'a');
});

test('V107-R33 places that are gone are skipped', () => {
  const h = new PlaceHistory(same);
  h.left('a');
  h.left('closed');
  assert.equal(h.goBack('c', (p) => p !== 'closed'), 'a');
});

test('V107-R33 the same place twice in a row is kept once', () => {
  const h = new PlaceHistory(same);
  h.left('a');
  h.left('a');
  assert.equal(h.goBack('b', any), 'a');
  assert.equal(h.goBack('a', any), null);
});
