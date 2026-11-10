import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const src = readFileSync(new URL('./ChatView.svelte', import.meta.url), 'utf8');

test('V110-B3 the REC chip sits outside the clipping .modes box and shrinks to a dot when narrow', () => {
  const rec = src.indexOf('class="mode rec"');
  const modes = src.indexOf('<div class="modes">');
  assert.ok(rec > 0 && modes > 0 && rec < modes, 'REC is a sibling before .modes');
  assert.match(src, /aria-label="Recording this chat"/);
  assert.match(src, /@container chat \(max-width: 360px\)[\s\S]*?\.rec-t\s*\{\s*display: none/);
  assert.match(src, /\.mode \{[^}]*flex: none/);
});
