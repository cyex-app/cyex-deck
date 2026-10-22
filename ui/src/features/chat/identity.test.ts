import assert from 'node:assert/strict';
import { test } from 'node:test';

import { hexColor, namedColor, smileyChoices } from './identity.ts';

test('V107-Mk free colors are #RRGGBB', () => {
  assert.equal(hexColor('#1e90ff'), '#1E90FF');
  assert.equal(hexColor(' 1E90FF '), '#1E90FF');
  assert.equal(hexColor('#1E90F'), null);
  assert.equal(hexColor('blue'), null);
});

test('V107-Mk a preset is known by name', () => {
  assert.equal(namedColor('#1e90ff'), 'Dodger blue');
  assert.equal(namedColor('#123456'), null);
  assert.equal(namedColor(null), null);
});

test('V107-Mk smiley styles show only with a choice', () => {
  const c = (style: 'robot' | 'glitch' | 'monkey') => ({ style, name: style, preview: [] });
  assert.deepEqual(smileyChoices(null), []);
  assert.deepEqual(smileyChoices({ current: 'robot', choices: [c('robot')] }), []);
  assert.equal(smileyChoices({ current: 'robot', choices: [c('robot'), c('glitch')] }).length, 2);
});
