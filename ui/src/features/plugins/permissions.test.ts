import assert from 'node:assert/strict';
import { test } from 'node:test';

import { menuSummary, permissionText } from './permissions.ts';

test('names the websites a plugin may reach', () => {
  assert.equal(
    permissionText({ type: 'Network', hosts: ['api.example.com', '*.example.org'] }).text,
    'Reach api.example.com, *.example.org',
  );
  assert.equal(permissionText({ type: 'Network', hosts: [] }).text, 'Reach any website');
});

test('changing chat says it reads chat too', () => {
  assert.match(permissionText({ type: 'ChangeChat' }).text, /Read chat/);
});

test('menu entries say where they show (S9, S11)', () => {
  const entry = (label: string, place: 'message' | 'chat' | 'name', group: string | null = null) => ({ label, place, group, lines: [] });
  assert.equal(
    menuSummary([entry('Forecast', 'chat', 'Weather'), entry('Who is this', 'name'), entry('Translate', 'message')]),
    'Weather › Forecast (chat), Who is this (names), Translate (lines)',
  );
  assert.equal(permissionText({ type: 'Tabs' }).text, 'Open channels in tabs and switch tabs');
});
