import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Panel } from '../../ipc/bindings/Panel';
import { linkLabel, linked, shows } from './panels.ts';

const panel = (p: Partial<Panel>): Panel => ({ title: null, image: null, link: null, body: [], ...p });
const none = new Set<string>();

test('a title alone shows', () => {
  assert.equal(shows(panel({ title: 'Rules' }), none), true);
});

test('an image alone shows', () => {
  assert.equal(shows(panel({ image: 'https://x/a.png' }), none), true);
});

test('a broken image alone hides the panel', () => {
  assert.equal(shows(panel({ image: 'https://x/a.png' }), new Set(['https://x/a.png'])), false);
});

test('text alone shows, even with a broken image', () => {
  const body = [{ type: 'paragraph' }] as unknown as Panel['body'];
  assert.equal(shows(panel({ body }), none), true);
  assert.equal(shows(panel({ body, image: 'https://x/a.png' }), new Set(['https://x/a.png'])), true);
});

test('an empty panel hides', () => {
  assert.equal(shows(panel({}), none), false);
});

test('linked follows the link', () => {
  assert.equal(linked(panel({ link: 'https://a.com' })), true);
  assert.equal(linked(panel({})), false);
});

test('link label names the host without www', () => {
  assert.equal(linkLabel(panel({ title: 'Merch', link: 'https://www.shop.com/x' })), 'Merch, opens shop.com');
  assert.equal(linkLabel(panel({ link: 'https://www.shop.com/x' })), 'Opens shop.com');
  assert.equal(linkLabel(panel({ link: 'https://store.example.org' })), 'Opens store.example.org');
});
