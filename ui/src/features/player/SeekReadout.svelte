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
    fade?.cancel();
    clearTimeout(timer);
    if (el) el.style.opacity = '1';
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
      if (el) el.style.opacity = '0';
    };
  }

  onDestroy(() => {
    clearTimeout(timer);
    fade?.cancel();
  });
</script>

<div class="pill" bind:this={el} aria-hidden="true">{text}</div>
<span class="said" aria-live="polite">{said}</span>

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
  .said {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
