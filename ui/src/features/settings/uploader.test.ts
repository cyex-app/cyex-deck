import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Uploader } from '../../ipc/bindings/Uploader.ts';
import { uploaderMissing } from './uploader.ts';

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
  assert.deepEqual(uploaderMissing(full), []);
});

test('names each empty required field', () => {
  assert.deepEqual(uploaderMissing({ ...full, url: '' }), ['Upload URL']);
  assert.deepEqual(uploaderMissing({ ...full, formField: '' }), ['Form field']);
  assert.deepEqual(uploaderMissing({ ...full, link: '' }), ['Image link']);
});

test('whitespace counts as empty', () => {
  assert.deepEqual(uploaderMissing({ ...full, url: '   ', link: '\t' }), ['Upload URL', 'Image link']);
});

test('headers and delete link may stay empty', () => {
  assert.deepEqual(uploaderMissing({ ...full, headers: '', deletionLink: ' ' }), []);
});
