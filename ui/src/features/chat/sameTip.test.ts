import assert from 'node:assert/strict';
import { test } from 'node:test';
import { sameTip } from './sameTip.ts';

const el = (dataset: Record<string, string>) => ({ dataset });

test('V111-02 a redrawn link is the same tip; another link or nothing is not', () => {
  assert.equal(sameTip(el({ link: 'https://a.b', host: 'a.b' }), el({ link: 'https://a.b', host: 'a.b' })), true);
  assert.equal(sameTip(el({ link: 'https://a.b' }), el({ link: 'https://c.d' })), false);
  assert.equal(sameTip(el({ emoji: '🙂' }), el({ emoji: '🙂' })), true);
  assert.equal(sameTip(el({ link: 'https://a.b' }), el({ tip: 'x' })), false);
  assert.equal(sameTip(el({}), el({})), false, 'nothing tipped');
});
