import assert from 'node:assert/strict';
import { test } from 'node:test';

import { sendLength } from './sendLength.ts';

test('a /me line counts its text against 491 (V107-N12a)', () => {
  assert.deepEqual(sendLength('/me waves'), { chars: 5, max: 491 });
  assert.deepEqual(sendLength('.me  hi '), { chars: 2, max: 491 });
  assert.deepEqual(sendLength('hello'), { chars: 5, max: 500 });
  assert.deepEqual(sendLength('/meow'), { chars: 5, max: 500 });
  assert.deepEqual(sendLength('😀😀'), { chars: 2, max: 500 });
});
