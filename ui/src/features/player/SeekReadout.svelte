<!-- V1061-04 The pill that names a seek ("-10s", "-0:42", "Live"). A held key restarts
     the timer, so it doesn't blink. -->
<script lang="ts">
  import { onDestroy } from 'svelte';

  import { reduced } from '../../lib/motion';

  let text = $state('');
  let said = $state('');
  let el = $state<HTMLElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let fade: Animation | undefined;

  export function show(t: string) {
    text = t;
    said = '';
    queueMicrotask(() => (said = t));
    // d-3 Again mid-fade: from where the fade is, a short ramp up, no pop.
    const from = fade && el ? Number(getComputedStyle(el).opacity) : 1;
    fade?.cancel();
    fade = undefined;
    clearTimeout(timer);
    if (el) {
      el.style.opacity = '1';
      if (from < 1 && !reduced()) el.animate([{ opacity: from }, { opacity: 1 }], { duration: 100, easing: 'ease-out' });
    }
    timer = setTimeout(hide, reduced() ? 900 : 700);
  }

  function hide() {
    if (!el) return;
    if (reduced()) {
      el.style.opacity = '0';
      return;
    }
    fade = el.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 250, easing: 'ease-out' });
    fade.onfinish = () => {
      fade = undefined;
      if (el) el.style.opacity = '0';
    };
  }

  onDestroy(() => {
    clearTimeout(timer);
    fade?.cancel();
  });
</script>

<div class="pill" bind:this={el} aria-hidden="true">{text}</div>
<span class="sr-only" aria-live="polite">{said}</span>

<style>
  .pill {
    position: absolute;
    top: 22%;
    left: 50%;
    transform: translateX(-50%);
    z-index: 3;
    padding: 4px 10px;
    border-radius: var(--radius);
    background: var(--scrim-strong);
    color: var(--on-video);
    font-size: 0.85rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    pointer-events: none;
    opacity: 0;
  }
</style>
