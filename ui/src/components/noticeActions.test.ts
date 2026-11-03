import assert from 'node:assert/strict';
import { test } from 'node:test';

import { cardAction, tabHolding } from './noticeActions.ts';

const tabs = [
  { id: 'a', logins: ['Xqc', 'other'] },
  { id: 'b', logins: ['xqc'] },
  { id: 'c', logins: ['mixed'] },
];

test('a tab that shows only the channel wins, case ignored', () => {
  assert.equal(tabHolding(tabs, 'XQC'), 'b');
  assert.equal(tabHolding(tabs, 'other'), 'a');
});

test('no tab gives null', () => {
  assert.equal(tabHolding(tabs, 'nobody'), null);
  assert.equal(tabHolding([], 'xqc'), null);
});

test('an existing tab: go to it and raise the window', () => {
  assert.deepEqual(cardAction('live', { kind: 'channel', login: 'mixed' }, tabs), {
    label: 'Go to tab',
    open: { kind: 'tab', login: 'mixed' },
    raise: true,
  });
});

test('no tab: open one in the background', () => {
  assert.deepEqual(cardAction('live', { kind: 'channel', login: 'new' }, tabs), {
    label: 'Open tab',
    open: { kind: 'background', login: 'new' },
    raise: false,
  });
});

test('the choice follows the tabs as they are now', () => {
  const o = { kind: 'channel', login: 'late' } as const;
  assert.equal(cardAction('live', o, tabs)?.label, 'Open tab');
  assert.equal(cardAction('live', o, [...tabs, { id: 'd', logins: ['late'] }])?.label, 'Go to tab');
});

test('only live cards about a channel have a button', () => {
  assert.equal(cardAction('drop', { kind: 'drops' }, tabs), null);
  assert.equal(cardAction('live', { kind: 'settings', page: 'x' }, tabs), null);
});
