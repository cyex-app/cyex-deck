<!-- ConfirmButton's multi-choice sibling: the question, then one button per way to go and
     Cancel, in a row. Esc or Cancel backs out; focus starts on Cancel so Enter never
     deletes by accident (V107-R37 rewards). -->
<script lang="ts">
  import Button from './Button.svelte';

  interface Choice {
    label: string;
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    run: () => void;
  }

  interface Props {
    ask: string;
    choices: Choice[];
    disabled?: boolean;
    oncancel: () => void;
  }

  let { ask, choices, disabled = false, oncancel }: Props = $props();

  function mount(node: HTMLElement) {
    node.querySelector<HTMLElement>('.cancel button')?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || e.defaultPrevented) return;
      e.preventDefault();
      e.stopPropagation();
      oncancel();
    };
    node.addEventListener('keydown', key);
    return { destroy: () => node.removeEventListener('keydown', key) };
  }
</script>

<div class="choices" role="group" aria-label={ask} use:mount>
  <span class="ask">{ask}</span>
  {#each choices as c (c.label)}
    <Button small variant={c.variant ?? 'secondary'} {disabled} onclick={c.run}>{c.label}</Button>
  {/each}
  <span class="cancel"><Button small variant="ghost" onclick={oncancel}>Cancel</Button></span>
</div>

<style>
  .choices {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }
  .ask {
    color: var(--text);
    font-size: 0.8rem;
  }
  .cancel {
    display: contents;
  }
</style>
