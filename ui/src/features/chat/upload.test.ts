import assert from 'node:assert/strict';
import { test } from 'node:test';

import { extOfMime, hostOf, imagePaths, uploadLine } from './upload.ts';

test('host of a url', () => {
  assert.equal(hostOf('https://I.Nuuls.com/upload'), 'i.nuuls.com');
  assert.equal(hostOf('http://127.0.0.1:8080/up'), '127.0.0.1:8080');
  assert.equal(hostOf('not a url'), '');
});

test('only image paths', () => {
  assert.deepEqual(imagePaths(['C:\\a\\x.PNG', 'b.txt', 'c.webp', 'noext']), ['C:\\a\\x.PNG', 'c.webp']);
});

test('extension from a clipboard type', () => {
  assert.equal(extOfMime('image/jpeg'), 'jpg');
  assert.equal(extOfMime('image/png'), 'png');
  assert.equal(extOfMime('image/bmp'), '');
});

test('our upload line', () => {
  assert.deepEqual(uploadLine('Uploaded https://x/a.png · delete: https://x/d/1'), { link: 'https://x/a.png', deletion: 'https://x/d/1' });
  assert.deepEqual(uploadLine('Uploaded https://x/a.png'), { link: 'https://x/a.png', deletion: '' });
  assert.equal(uploadLine('Uploaded a b'), null);
  assert.equal(uploadLine('Something else'), null);
});
