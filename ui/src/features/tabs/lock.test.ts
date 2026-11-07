import assert from 'node:assert/strict';
import { test } from 'node:test';
import { dragBlocked, dropBlocked } from './lock.ts';

test('unlocked blocks nothing (V109-U05)', () => {
  assert.equal(dragBlocked(false, { kind: 'tab' }), false);
  assert.equal(dropBlocked(false, { kind: 'tab' }, { kind: 'bar' }), false);
  assert.equal(dropBlocked(false, { kind: 'tile' }, { kind: 'bar' }), false);
});

test('locked stops dragging a tab', () => {
  assert.equal(dragBlocked(true, { kind: 'tab' }), true);
  for (const kind of ['tab', 'group', 'bar', 'tile']) assert.equal(dropBlocked(true, { kind: 'tab' }, { kind }), true);
});

test('locked leaves tiles, panels and rows alone, except into the bar', () => {
  for (const kind of ['tile', 'panel', 'row']) assert.equal(dragBlocked(true, { kind }), false);
  assert.equal(dropBlocked(true, { kind: 'tile' }, { kind: 'tile' }), false);
  assert.equal(dropBlocked(true, { kind: 'tile' }, { kind: 'bar' }), true);
  assert.equal(dropBlocked(true, { kind: 'tile' }, { kind: 'tab' }), true);
  assert.equal(dropBlocked(true, { kind: 'panel' }, { kind: 'panel' }), false);
  assert.equal(dropBlocked(true, { kind: 'row' }, { kind: 'row' }), false);
});
