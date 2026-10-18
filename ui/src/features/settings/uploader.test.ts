import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Uploader } from '../../ipc/bindings/Uploader.ts';
import { uploaderMissingKeys } from './uploader.ts';

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
