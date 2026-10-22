<!-- V107-Mr Unseen mentions on a tab, a folded group or a switcher row: an accent dot and
     the count, calm (no blinking). Accent text, which Rust keeps readable on the page. -->
<script lang="ts">
  import { formatNumber } from '../../lib/format';

  let { count }: { count: number } = $props();

  const shown = $derived(count > 99 ? '99+' : count > 0 ? formatNumber(count) : '');
</script>

<span class="ping" aria-hidden="true">{shown}</span>
<span class="sr-only">, {count === 1 ? '1 mention' : count > 1 ? `${formatNumber(count)} mentions` : 'mentioned'}</span>

<style>
  .ping {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex: none;
    font-size: 11px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    color: var(--accent-text);
  }
  .ping::before {
    content: '';
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: currentColor;
  }
</style>
