import assert from 'node:assert/strict';
import { test } from 'node:test';

import { opensItself, startState } from './updateState.ts';

const u = (version: string) => ({ version, prerelease: false, notesUrl: '', canInstall: true });

test('B4 later comes from Rust after a reload', () => {
  assert.deepEqual(startState({ update: u('0.1.6'), later: true, checked: 5 }, undefined), {
    update: u('0.1.6'),
    later: true,
    checked: 5,
  });
  assert.deepEqual(startState(null, undefined), { update: null, later: false, checked: null });
});

test('B4 an update that came in while reading wins', () => {
  assert.deepEqual(startState({ update: u('0.1.6'), later: true, checked: null }, u('0.1.7')), {
    update: u('0.1.7'),
    later: true,
    checked: null,
  });
  assert.deepEqual(startState({ update: u('0.1.6'), later: false, checked: 9 }, null), {
    update: null,
    later: false,
    checked: 9,
  });
});

test('B4 the pop-over stays shut after later', () => {
  assert.equal(opensItself('0.1.6', null, true), false);
  assert.equal(opensItself('0.1.6', null, false), true);
  assert.equal(opensItself('0.1.6', '0.1.6', false), false);
  assert.equal(opensItself(undefined, null, false), false);
});
