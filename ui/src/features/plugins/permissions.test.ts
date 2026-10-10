import assert from 'node:assert/strict';
import { test } from 'node:test';

import { permissionText } from './permissions.ts';

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
