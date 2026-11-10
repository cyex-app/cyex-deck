<!-- Filters (F3) in Chatterino's language, picked per chat from its menu, and
     near-duplicate messages (F6). -->
<script lang="ts">
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { FilterDef } from '../../../ipc/bindings/FilterDef';
  import type { FilterName } from '../../../ipc/bindings/FilterName';
  import type { Similarity } from '../../../ipc/bindings/Similarity';
  import { rulesIpc } from '../../../ipc/rules';
  import { settings } from '../../../lib/settings.svelte';
  import SectionHead from '../SectionHead.svelte';
  import type { Field } from '../rules/fields';
  import RuleList from '../rules/RuleList.svelte';
  import { CAPS } from '../rules/caps';
  import NumberField from '../../../components/NumberField.svelte';
  import { tooltip } from '../../../components/tooltip.svelte';
  import { untrack } from 'svelte';
  import { settingsPlace } from '../place.svelte';

  const h = $derived(settings.v.highlights);
  const s = $derived(h.similar);
  const sim = (patch: Partial<Similarity>) => void settings.patch('highlights', { similar: patch });

  const fields: Field<FilterDef>[] = [
    { key: 'name', label: 'Name', kind: 'text', placeholder: 'Name', main: true },
    {
      key: 'expr',
      label: 'Filter',
      kind: 'area',
      placeholder: 'e.g. author.subbed && message.content contains "hi"',
      wide: true,
    },
  ];

  /** Why each filter doesn't read, by text. */
  let problems = $state<Record<string, string | null>>({});
  let names = $state.raw<FilterName[]>([]);
  let showNames = $state(settingsPlace.open('filters', 'names'));
  $effect(() => settingsPlace.setOpen('filters', 'names', showNames));
  if (untrack(() => showNames)) rulesIpc.filterNames().then((n) => (names = n)).catch(() => {});

  $effect(() => {
    for (const f of h.filters) {
      if (f.expr in problems) continue;
      const expr = f.expr;
      rulesIpc
        .checkFilter(expr)
        .then((p) => (problems = { ...problems, [expr]: p }))
        .catch(() => {});
    }
  });

  function toggleNames() {
    showNames = !showNames;
    if (showNames && !names.length) rulesIpc.filterNames().then((n) => (names = n)).catch(() => {});
  }

  const id = () => crypto.randomUUID().slice(0, 8);
</script>

<!-- b-15 One line, so exactly one space sits before the button. -->
<SectionHead title="Filters" level={2} docs="filters"
  >A chat shows only what passes its filters. Pick them from the chat’s menu. Same language as Chatterino. <button
    type="button"
    class="doclink"
    onclick={toggleNames}>{showNames ? 'Hide' : 'Show'} what you can use</button
  ></SectionHead
>
{#if showNames}
  <div class="names">
    {#each names as n (n.name)}<code use:tooltip={n.kind}>{n.name}</code>{/each}
  </div>
{/if}
<RuleList
  setting="highlights.filters"
  items={h.filters}
  max={CAPS.filters}
  {fields}
  layout="split"
  make={() => ({ id: id(), name: '', expr: '' })}
  onchange={(filters) => void settings.patch('highlights', { filters })}
  problem={(f) => (f.expr.trim() ? (problems[f.expr] ?? null) : null)}
  addLabel="Add filter"
/>

<SectionHead title="Near-duplicates">Spot messages that repeat what was just said.</SectionHead>
<Row setting="highlights.similar.enabled" label="Mark near-duplicates" hint="Messages much like one sent just before. Ctrl+H hides or shows them.">
  <Toggle label="Mark near-duplicates" checked={s.enabled} onchange={(v) => sim({ enabled: v })} />
</Row>
<Row setting="highlights.similar.hide" label="Hide them" hint="Instead of greying them out.">
  <Toggle label="Hide them" checked={s.hide} onchange={(v) => sim({ hide: v })} />
</Row>
<Row setting="highlights.similar.threshold" label="How alike" hint="Percent. Higher only catches closer copies.">
  <NumberField label="How alike" steppers min={50} max={100} width="80px" unit="%" value={s.threshold} onchange={(n) => sim({ threshold: n })} />
</Row>
<Row setting="highlights.similar.maxAgeSecs" label="Within" hint="Seconds back to compare with.">
  <NumberField label="Within" steppers min={1} max={600} width="80px" unit="s" value={s.maxAgeSecs} onchange={(n) => sim({ maxAgeSecs: n })} />
</Row>
<Row setting="highlights.similar.depth" label="Messages back" hint="How many earlier messages to compare with.">
  <NumberField label="Messages back" steppers width="80px" value={s.depth} min={1} max={5} onchange={(v) => sim({ depth: v })} />
</Row>
<Row setting="highlights.similar.sameUser" label="Only from the same person" hint="Ignores matches between different people.">
  <Toggle label="Only from the same person" checked={s.sameUser} onchange={(v) => sim({ sameUser: v })} />
</Row>
<Row setting="highlights.similar.own" label="Check your own messages" hint="Your messages can be marked too.">
  <Toggle label="Check your own messages" checked={s.own} onchange={(v) => sim({ own: v })} />
</Row>

<style>
  /* V110-F35 Looks like the Docs link beside it (SectionHead). */
  .doclink {
    padding: 0;
    background: none;
    border: 0;
    color: var(--muted);
    font: inherit;
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    text-underline-offset: 2px;
    cursor: pointer;
  }
  .doclink:hover {
    color: var(--text);
    text-decoration-color: currentColor;
  }
  .names {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 10px;
  }
  code {
    padding: 1px 5px;
    background: var(--surface);
    border-radius: var(--radius);
    font-size: 0.75rem;
  }
</style>
