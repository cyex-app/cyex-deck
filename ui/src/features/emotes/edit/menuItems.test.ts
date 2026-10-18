import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { MenuItem } from '../../../components/menuItems.ts';
import { editEntries } from './menuItems.ts';

const h = { add: () => {}, rename: () => {}, remove: () => {} };
const t = (channel: string, extra: object = {}) => ({ setId: 's' + channel, channel, own: false, viewed: false, blocked: null, ...extra });
const opts = (o: object) => ({ state: 'ready', ownName: 'x', add: null, here: null, ...o }) as never;
const items = (list: ReturnType<typeof editEntries>) => list.filter((e): e is MenuItem => e !== 'separator');

test('nothing when not ready or empty', () => {
  assert.deepEqual(editEntries(null, h), []);
  assert.deepEqual(editEntries(opts({ state: 'loading' }), h), []);
  assert.deepEqual(editEntries(opts({}), h), []);
});

test('one target is a plain item', () => {
  const l = editEntries(opts({ add: { blocked: null, targets: [t('kuni')] } }), h);
  assert.equal(l[0], 'separator');
  const [i] = items(l);
  assert.equal(i.label, 'Add to kuni');
  assert.equal(i.disabled, false);
  assert.equal(i.items, undefined);
});

test('one blocked target explains itself', () => {
  const [i] = items(editEntries(opts({ add: { blocked: null, targets: [t('kuni', { blocked: 'already' })] } }), h));
  assert.equal(i.disabled, true);
  assert.equal(i.note, 'Already there');
});

test('several targets make a submenu in Rust order with hints', () => {
  const targets = [t('b', { viewed: true }), t('a', { own: true }), t('c', { blocked: 'full' })];
  const [i] = items(editEntries(opts({ add: { blocked: null, targets } }), h));
  assert.equal(i.label, 'Add to 7TV channel');
  const sub = items(i.items ?? []);
  assert.deepEqual(sub.map((x) => x.label), ['b', 'a', 'c']);
  assert.deepEqual(sub.map((x) => x.hint), ['This chat', 'Yours', undefined]);
  assert.equal(sub[2].disabled, true);
  assert.equal(sub[2].note, 'Full');
});

test('a whole blocked item greys', () => {
  const [i] = items(editEntries(opts({ add: { blocked: 'private', targets: [t('a'), t('b')] } }), h));
  assert.equal(i.disabled, true);
  assert.equal(i.note, 'Private emote');
});

test('here only and both', () => {
  const here = { setId: 's', channel: 'kuni', alias: 'KEKW' };
  const only = items(editEntries(opts({ here }), h));
  assert.deepEqual(only.map((x) => x.label), ['Rename…', 'Remove from kuni']);
  const both = items(editEntries(opts({ here, add: { blocked: null, targets: [t('kuni')] } }), h));
  assert.equal(both.length, 3);
});

test('rc-3 already in some channels, the rest full: says it is there', () => {
  const targets = [t('a', { blocked: 'already' }), t('b', { blocked: 'already' }), t('c', { blocked: 'full' })];
  const [i] = items(editEntries(opts({ add: { blocked: 'nowhere', targets } }), h));
  assert.equal(i.disabled, true);
  assert.equal(i.note, 'Already in your channels');
  const full = [t('a', { blocked: 'full' }), t('b', { blocked: 'private' })];
  const [j] = items(editEntries(opts({ add: { blocked: 'nowhere', targets: full } }), h));
  assert.equal(j.note, 'No room in your channels');
});
