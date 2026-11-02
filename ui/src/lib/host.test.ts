import assert from 'node:assert/strict';
import { test } from 'node:test';
import { hostOf } from './host.ts';

test('V108-F34 hostOf drops only a real www. label', () => {
  assert.equal(hostOf('https://www.twitch.tv/x'), 'twitch.tv');
  assert.equal(hostOf('https://www1.example.com'), 'www1.example.com');
  assert.equal(hostOf('https://wwwide.com'), 'wwwide.com');
});

test('V108-F34 hostOf gives nothing for a non-url', () => {
  assert.equal(hostOf('nope'), '');
});
