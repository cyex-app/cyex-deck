import assert from 'node:assert/strict';
import { test } from 'node:test';

import { previewPic } from './previewPic.ts';

const proxy = 'https://braize.pajlada.com/chatterino/thumbnail/https%3A%2F%2Fi.ytimg.com%2Fa.jpg';

test('V110-F75 the resolver proxy gets through', () => {
  assert.equal(previewPic(proxy), proxy);
});

const own = 'https://embed.cyex.app/v1/img?u=https%3A%2F%2Fi.ytimg.com%2Fa.jpg';

test('V110-U70 pictures from our own resolver get through', () => {
  assert.equal(previewPic(own), own);
});

test('V110-U70 lookalikes of our resolver are refused', () => {
  for (const u of [
    'http://embed.cyex.app/v1/img?u=x',
    'https://embed.cyex.app.evil.io/v1/img?u=x',
    'https://embed.cyex.app@evil.io/v1/img?u=x',
    'https://user:pw@embed.cyex.app/v1/img?u=x',
    'https://embed.cyex.app:8443/v1/img?u=x',
    'https://embed.cyex.app/v1/img',
    'https://embed.cyex.app/v1/img?u=',
    'https://embed.cyex.app/v1/img/x?u=x',
    'https://embed.cyex.app/v1/card?url=x',
    'https://embed.cyex.app/other?u=x',
    'https://cyex.app/v1/img?u=x',
    'https://evil.io/?u=https://embed.cyex.app/v1/img?u=x',
  ]) {
    assert.equal(previewPic(u), undefined, u);
  }
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
