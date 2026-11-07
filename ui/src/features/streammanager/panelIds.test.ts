import assert from 'node:assert/strict';
import { test } from 'node:test';

import { leaf, leaves, split } from '../../lib/panels.ts';
import { savedLayout } from './panelIds.ts';

test('V109-C04 a layout saved with moved panels loads without them', () => {
  const saved = split(
    'row',
    [
      split('column', [leaf('stream'), leaf('followers')], [0.5, 0.5]),
      split('column', [leaf('rewards'), leaf('clips'), leaf('videos'), leaf('modset')]),
      leaf('rolesmgr'),
      leaf('chat'),
    ],
    [0.3, 0.3, 0.2, 0.2],
  );
  const out = savedLayout(saved);
  assert.deepEqual(leaves(out), ['stream', 'rolesmgr', 'chat']);
  // The emptied column folds away; what's left shares the row.
  assert.ok(out && 'dir' in out && out.parts.length === 3);
  assert.ok(Math.abs(out.sizes.reduce((a, b) => a + b, 0) - 1) < 1e-9);
});

test('V109-C04 only moved or unknown panels: nothing saved is left', () => {
  assert.equal(savedLayout(split('column', [leaf('followers'), leaf('nope')])), null);
  assert.equal(savedLayout(leaf('clips')), null);
  assert.equal(savedLayout(null), null);
  assert.equal(savedLayout(undefined), null);
});

test('V109-C04 a layout with only kept panels stays as it was', () => {
  const saved = split('row', [leaf('stream'), leaf('roles'), leaf('chat')], [0.5, 0.25, 0.25]);
  assert.deepEqual(savedLayout(saved), saved);
});
