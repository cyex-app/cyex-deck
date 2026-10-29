import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Uploader } from '../../ipc/bindings/Uploader.ts';
import { turnsOnWithUrl, uploaderMissingKeys, validUploadUrl } from './uploader.ts';

const full: Uploader = {
  enabled: false,
  confirm: true,
  url: 'https://h.example/up',
  formField: 'file',
  headers: '',
  link: '{data.link}',
  deletionLink: '',
};

test('nothing missing when the three required fields are filled (V106-14)', () => {
  assert.deepEqual(uploaderMissingKeys(full), []);
});

test('names each empty required field', () => {
  assert.deepEqual(uploaderMissingKeys({ ...full, url: '' }), ['url']);
  assert.deepEqual(uploaderMissingKeys({ ...full, formField: '' }), ['formField']);
  assert.deepEqual(uploaderMissingKeys({ ...full, link: '' }), ['link']);
});

test('whitespace counts as empty', () => {
  assert.deepEqual(uploaderMissingKeys({ ...full, url: '   ', link: '\t' }), ['url', 'link']);
});

test('headers and delete link may stay empty', () => {
  assert.deepEqual(uploaderMissingKeys({ ...full, headers: '', deletionLink: ' ' }), []);
});

test('validUploadUrl takes http(s) addresses with a host (V108-C07)', () => {
  assert.equal(validUploadUrl('https://h.example/up'), true);
  assert.equal(validUploadUrl(' http://localhost:8080/up '), true);
  for (const bad of ['', '   ', 'h.example/up', 'ftp://h.example', 'https://', 'https://a b.example', 'javascript:alert(1)', 'not a url'])
    assert.equal(validUploadUrl(bad), false, bad);
});

test('a valid address turns uploading on only from empty or invalid, while off (V108-C07)', () => {
  assert.equal(turnsOnWithUrl('', 'https://h.example', false), true);
  assert.equal(turnsOnWithUrl('nope', 'https://h.example', false), true);
  assert.equal(turnsOnWithUrl('https://a.example', 'https://b.example', false), false);
  assert.equal(turnsOnWithUrl('', 'https://h.example', true), false);
  assert.equal(turnsOnWithUrl('', 'nope', false), false);
  assert.equal(turnsOnWithUrl('https://a.example', '', false), false);
});
