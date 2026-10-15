import assert from 'node:assert/strict';
import { test } from 'node:test';

import { sinkFor, windowsName } from './sink.ts';

const out = (deviceId: string, label: string) => ({ kind: 'audiooutput', deviceId, label });
// Labels as WebView2 155 listed them on the test PC.
const LIST = [
  out('default', 'Default - System (TC-HELICON GoXLR)'),
  out('communications', 'Communications - System (TC-HELICON GoXLR)'),
  out('d57d', 'Speakers (FiiO K5 Pro) (2972:0047)'),
  out('3377', 'Music (TC-HELICON GoXLR)'),
  out('c3d6', 'System (TC-HELICON GoXLR)'),
];

test('the USB id goes, the name stays', () => {
  assert.equal(windowsName('Speakers (FiiO K5 Pro) (2972:0047)'), 'Speakers (FiiO K5 Pro)');
  assert.equal(windowsName('Music (TC-HELICON GoXLR)'), 'Music (TC-HELICON GoXLR)');
});

test('finds the device by its Windows name, never the default entries', () => {
  assert.equal(sinkFor(LIST, 'Speakers (FiiO K5 Pro)'), 'd57d');
  assert.equal(sinkFor(LIST, 'System (TC-HELICON GoXLR)'), 'c3d6');
});

test('a device that isn’t there plays on the default', () => {
  assert.equal(sinkFor(LIST, 'Headphones'), '');
  assert.equal(sinkFor([out('', '')], 'Music (TC-HELICON GoXLR)'), '');
});
