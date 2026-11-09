import assert from 'node:assert/strict';
import { test } from 'node:test';

import { previewPic } from './previewPic.ts';

const proxy = 'https://braize.pajlada.com/chatterino/thumbnail/https%3A%2F%2Fi.ytimg.com%2Fa.jpg';

test('V110-F75 the resolver proxy gets through', () => {
  assert.equal(previewPic(proxy), proxy);
});

test('V110-F75 everything else is refused', () => {
    for (const u of [
      'https://i.ytimg.com/vi/a/mqdefault.jpg',
      'http://braize.pajlada.com/chatterino/thumbnail/x',
      'https://braize.pajlada.com.evil.io/chatterino/thumbnail/x',
      'https://braize.pajlada.com@evil.io/chatterino/thumbnail/x',
      'https://user:pw@braize.pajlada.com/chatterino/thumbnail/x',
      'https://braize.pajlada.com:8443/chatterino/thumbnail/x',
      'https://braize.pajlada.com/other/x',
      'https://evil.io/?u=https://braize.pajlada.com/chatterino/thumbnail/x',
      'data:image/png;base64,AA',
      'blob:https://braize.pajlada.com/x',
      '//braize.pajlada.com/chatterino/x',
      'not a url',
      '',
      null,
      undefined,
    ]) {
      assert.equal(previewPic(u), undefined, String(u));
    }
});
