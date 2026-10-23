<!-- V107-R19 Circle crop for a new profile picture: drag to move, wheel or slider to zoom.
     Saved as a 240×240 PNG, as Twitch's own editor does. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import { type Crop, OUT_SIZE, ZOOM_MAX, coverScale, drag, initial, source, zoomTo } from './crop';

  let { file, ondone, onclose }: { file: File; ondone: (png: Uint8Array) => void; onclose: () => void } = $props();

  const VIEW = 240;

  let img = $state.raw<HTMLImageElement | null>(null);
  let crop = $state.raw<Crop>({ x: 0, y: 0, scale: 1 });
  let failed = $state(false);
  let saving = $state(false);
  let from: { x: number; y: number } | null = null;

  const zoom = $derived(img ? crop.scale / coverScale(img.naturalWidth, img.naturalHeight, VIEW) : 1);

  $effect(() => {
    const url = URL.createObjectURL(file);
    const el = new Image();
    el.onload = () => {
      img = el;
      crop = initial(el.naturalWidth, el.naturalHeight, VIEW);
    };
    el.onerror = () => (failed = true);
    el.src = url;
    return () => URL.revokeObjectURL(url);
  });

  function setZoom(z: number) {
    if (img) crop = zoomTo(crop, z, img.naturalWidth, img.naturalHeight, VIEW);
  }

  function down(e: PointerEvent) {
    from = { x: e.clientX, y: e.clientY };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function move(e: PointerEvent) {
    if (!from || !img) return;
    crop = drag(crop, e.clientX - from.x, e.clientY - from.y, img.naturalWidth, img.naturalHeight, VIEW);
    from = { x: e.clientX, y: e.clientY };
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    setZoom(zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1));
  }

  async function save() {
    if (!img) return;
    saving = true;
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = OUT_SIZE;
    const ctx = canvas.getContext('2d');
    const s = source(crop, VIEW);
    ctx?.drawImage(img, s.sx, s.sy, s.size, s.size, 0, 0, OUT_SIZE, OUT_SIZE);
    const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, 'image/png'));
    saving = false;
    if (blob) ondone(new Uint8Array(await blob.arrayBuffer()));
    else failed = true;
  }
</script>

<Dialog title="New profile picture" width={320} {onclose} onsubmit={save}>
  {#if failed}
    <p class="err">Couldn’t open that image.</p>
  {:else}
    <div
      class="view"
      style:width="{VIEW}px"
      style:height="{VIEW}px"
      role="presentation"
      onpointerdown={down}
      onpointermove={move}
      onpointerup={() => (from = null)}
      onpointercancel={() => (from = null)}
      onwheel={wheel}
    >
      {#if img}
        <img
          src={img.src}
          alt=""
          draggable="false"
          style:width="{img.naturalWidth * crop.scale}px"
          style:height="{img.naturalHeight * crop.scale}px"
          style:transform="translate({crop.x}px, {crop.y}px)"
        />
      {/if}
      <div class="ring"></div>
    </div>
    <input
      class="zoom"
      type="range"
      min="1"
      max={ZOOM_MAX}
      step="0.01"
      value={zoom}
      aria-label="Zoom"
      oninput={(e) => setZoom(+e.currentTarget.value)}
    />
  {/if}
  {#snippet footer()}
    <Button variant="ghost" onclick={onclose}>Cancel</Button>
    <Button variant="primary" disabled={!img || saving || failed} onclick={save}>Use picture</Button>
  {/snippet}
</Dialog>

<style>
  .view {
    position: relative;
    margin: 4px auto 0;
    overflow: hidden;
    border-radius: var(--radius);
    background: var(--bg);
    cursor: grab;
    touch-action: none;
    user-select: none;
  }
  .view:active {
    cursor: grabbing;
  }
  img {
    position: absolute;
    top: 0;
    left: 0;
    max-width: none;
    transform-origin: 0 0;
    pointer-events: none;
  }
  .ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    box-shadow: 0 0 0 999px var(--scrim);
    outline: 1px solid var(--on-video-faint);
    pointer-events: none;
  }
  .zoom {
    display: block;
    width: 240px;
    margin: 14px auto 2px;
    accent-color: var(--accent-line);
  }
  .err {
    color: var(--red);
  }
</style>
