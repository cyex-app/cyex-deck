import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  asPatch,
  clampSizes,
  dragSizes,
  ensure,
  fitTree,
  insert,
  layout,
  leaf,
  leaves,
  MARGIN,
  matchSplit,
  move,
  type Node,
  normalize,
  remove,
  restore,
  setSizes,
  split,
  type Split,
  splitKey,
  stacked,
} from './panels.ts';

const near = (a: number[], b: number[]) => a.every((x, i) => Math.abs(x - b[i]) < 1e-6);

test('a panel dropped on an edge lands on that side', () => {
  // V102-23 as before: the chat dropped on the stream's left goes left of it.
  const t = split('row', [leaf('stream'), leaf('chat')], [0.72, 0.28]);
  const moved = move(t, 'chat', 'stream', 'left') as Split;
  assert.deepEqual(leaves(moved), ['chat', 'stream']);
  // It keeps its part of the room.
  assert.ok(near(moved.sizes, [0.28, 0.72]));
  // V103-05 Any panel, not just chat and stream: the mod view under the stream.
  const three = insert(t, 'stream', 'bottom', 'modview');
  assert.deepEqual(three, split('row', [split('column', [leaf('stream'), leaf('modview')]), leaf('chat')], [0.72, 0.28]));
  // Next to a sibling going the same way it joins that split and takes its share from the target.
  const four = insert(three, 'chat', 'right', 'x', 0.5) as Split;
  assert.equal(four.parts.length, 3);
  assert.ok(near(four.sizes, [0.72, 0.14, 0.14]));
});

test('dropping on itself changes nothing, the whole layout takes an edge with no target', () => {
  const t = split('column', [leaf('a'), leaf('b')]);
  assert.equal(move(t, 'a', 'a', 'left'), t);
  assert.deepEqual(leaves(insert(t, null, 'left', 'c')), ['c', 'a', 'b']);
  assert.deepEqual(insert(null, 'a', 'top', 'c'), leaf('c'));
  // From the closed-panels corner (not in the layout): half of the panel it lands on.
  assert.ok(near((move(t, 'c', 'b', 'bottom') as Split).sizes, [0.5, 0.25, 0.25]));
});

test('removing folds splits away and joins splits going the same way', () => {
  const t = split('row', [leaf('a'), split('column', [leaf('b'), split('row', [leaf('c'), leaf('d')])])], [0.5, 0.5]);
  const r = remove(t, 'b') as Split;
  assert.equal(r.dir, 'row');
  assert.deepEqual(leaves(r), ['a', 'c', 'd']);
  assert.ok(near(r.sizes, [0.5, 0.25, 0.25]));
  assert.equal(remove(leaf('a'), 'a'), null);
  // Repeats and empty splits go, as in Rust's sanitize.
  assert.deepEqual(normalize(split('row', [leaf('a'), leaf('a'), split('column', [])])), leaf('a'));
});

test('a closed panel comes back next to its default neighbour', () => {
  // V103-06, V103-07 The mod view's default with every panel; the stream starts closed.
  const full = split('row', [
    split('column', [leaf('stream'), leaf('chat')], [0.4, 0.6]),
    split('column', [leaf('activity'), leaf('actions')]),
  ]);
  const now = remove(full, 'stream');
  const back = restore(now, 'stream', full);
  assert.deepEqual(leaves(back), ['stream', 'chat', 'activity', 'actions']);
  assert.ok(near((((back as Split).parts[0]) as Split).sizes, [0.4, 0.6]));
  // Its neighbour gone too: the next one out.
  const bare = remove(remove(full, 'stream'), 'chat');
  assert.deepEqual(leaves(restore(bare, 'chat', full)), ['chat', 'activity', 'actions']);
  // Nothing left at all: it's the whole layout.
  assert.deepEqual(restore(null, 'actions', full), leaf('actions'));
  // Already there: unchanged; several at once.
  assert.equal(restore(full, 'chat', full), full);
  assert.deepEqual(leaves(ensure(leaf('actions'), ['chat', 'stream'], full)), ['stream', 'chat', 'actions']);
});

test('shares keep every part at its least size where there is room', () => {
  assert.ok(near(clampSizes([0.72, 0.28], [240, 240], 700), [460 / 700, 240 / 700]));
  assert.ok(near(clampSizes([0.5, 0.5], [100, 100], 1000), [0.5, 0.5]));
  // Not enough room for all: each gets its part of what there is.
  assert.ok(near(clampSizes([0.9, 0.1], [300, 100], 200), [0.75, 0.25]));
  // Bad shares become even.
  assert.ok(near(clampSizes([NaN, 0], [0, 0], 100), [0.5, 0.5]));
});

const MIN: Record<string, { w: number; h: number }> = {
  stream: { w: 240, h: 135 },
  chat: { w: 240, h: 200 },
  modview: { w: 320, h: 260 },
};
const min = (id: string) => MIN[id] ?? { w: 100, h: 100 };

test('a split too small its own way turns round, and turns back with some room to spare', () => {
  const t = split('row', [leaf('stream'), leaf('chat')], [0.72, 0.28]);
  const roomy = fitTree(t, { w: 1200, h: 700 }, { min, turn: true });
  assert.ok(roomy.fits && roomy.turned.size === 0);
  assert.equal((roomy.tree as Split).dir, 'row');
  const narrow = fitTree(t, { w: 420, h: 700 }, { min, turn: true });
  assert.equal((narrow.tree as Split).dir, 'column');
  assert.ok(narrow.turned.has(splitKey(t as Split)));
  // Just past the limit it stays turned; past the margin it goes back.
  const edge = 480;
  assert.equal((fitTree(t, { w: edge + 10, h: 700 }, { min, turn: true, turned: narrow.turned }).tree as Split).dir, 'column');
  assert.equal((fitTree(t, { w: edge + MARGIN, h: 700 }, { min, turn: true, turned: narrow.turned }).tree as Split).dir, 'row');
  // Without turning (mod view, stream manager) it scrolls at its least size instead.
  const tight = fitTree(t, { w: 420, h: 700 }, { min });
  assert.ok(!tight.fits);
  assert.deepEqual(tight.box, { w: 480, h: 700 });
});

test('a turned split takes the shares it is given', () => {
  const t = split('row', [leaf('stream'), leaf('chat')], [0.72, 0.28]);
  const f = fitTree(t, { w: 420, h: 700 }, { min, turn: true, turnedSizes: () => [0.6, 0.4] });
  assert.ok(near((f.tree as Split).sizes, [0.6, 0.4]));
});

test('gaps and padding count toward the room panels need', () => {
  const t = split('row', [leaf('a'), leaf('b')]);
  // Two 100 px panels, an 8 px gap and 8 px padding: 224 wide.
  assert.ok(fitTree(t, { w: 224, h: 116 }, { min, gap: 8, pad: 8 }).fits);
  const f = fitTree(t, { w: 200, h: 116 }, { min, gap: 8, pad: 8 });
  assert.ok(!f.fits);
  assert.deepEqual(f.box, { w: 224, h: 116 });
  const laid = layout(t, { w: 224, h: 116 }, 8, 8);
  assert.deepEqual(laid.panels.get('a'), { x: 8, y: 8, w: 100, h: 100 });
  assert.deepEqual(laid.panels.get('b'), { x: 116, y: 8, w: 100, h: 100 });
  assert.equal(laid.dividers.length, 1);
  assert.equal(laid.dividers[0].at, 112);
});

test('a narrow mod view stacks its panels one under the other', () => {
  const t = split('row', [leaf('chat'), split('column', [leaf('a'), leaf('b')])]);
  const s = stacked(t, 320, (id) => (id === 'chat' ? 400 : 200), 8, 8);
  assert.deepEqual(leaves(s.tree), ['chat', 'a', 'b']);
  assert.equal(s.box.h, 400 + 200 + 200 + 2 * 8 + 2 * 8);
  const laid = layout(s.tree, s.box, 8, 8);
  assert.equal(laid.panels.get('chat')!.h, 400);
  assert.equal(laid.panels.get('b')!.y, 8 + 400 + 8 + 200 + 8);
});

test('a divider drag finds its split in the stored tree, not when fitting turned it', () => {
  const user = split('row', [leaf('modview'), split('column', [leaf('stream'), leaf('chat')])]);
  // The mod view closed: the shown tree is the inner split alone.
  const shown = remove(user, 'modview')!;
  assert.deepEqual(matchSplit(user, shown, []), [1]);
  assert.deepEqual(matchSplit(user, user, []), []);
  const turned: Node = { ...(shown as Split), dir: 'row' };
  assert.equal(matchSplit(user, turned, []), null);
  const resized = setSizes(user, [1], [0.6, 0.4]);
  assert.ok(near(((resized as Split).parts[1] as Split).sizes, [0.6, 0.4]));
});

test('dragging a divider keeps both parts at their least size', () => {
  const s = split('row', [leaf('a'), leaf('b'), leaf('c')], [0.25, 0.25, 0.5]) as Split;
  const rect = { x: 0, y: 0, w: 1000, h: 500 };
  assert.ok(near(dragSizes(s, rect, 1, 400, [100, 100, 100]), [0.4, 0.1, 0.5]));
  // Past the next part's least size it stops there.
  assert.ok(near(dragSizes(s, rect, 1, 480, [100, 100, 100]), [0.4, 0.1, 0.5]));
  assert.ok(near(dragSizes(s, rect, 2, 0, [100, 100, 100]), [0.25, 0.1, 0.65]));
});

test('a layout patch clears the other kind of node', () => {
  assert.deepEqual(asPatch(leaf('a')), { panel: 'a', dir: null, parts: null, sizes: null });
  assert.deepEqual(asPatch(split('row', [leaf('a'), leaf('b')])), {
    dir: 'row',
    parts: [leaf('a'), leaf('b')],
    sizes: [0.5, 0.5],
    panel: null,
  });
  assert.equal(asPatch(null), null);
});
