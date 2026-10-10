<!-- Filters (F3) in Chatterino's language, picked per chat from its menu, and
     near-duplicate messages (F6). -->
<script lang="ts">
  import Row from '../../../components/Row.svelte';
  import Stepper from '../../../components/Stepper.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { FilterDef } from '../../../ipc/bindings/FilterDef';
  import type { FilterName } from '../../../ipc/bindings/FilterName';
  import type { Similarity } from '../../../ipc/bindings/Similarity';
  import { rulesIpc } from '../../../ipc/rules';
  import { settings } from '../../../lib/settings.svelte';
  import type { Field } from '../rules/fields';
  import RuleList from '../rules/RuleList.svelte';

  const h = $derived(settings.v.highlights);
  const s = $derived(h.similar);
  const sim = (patch: Partial<Similarity>) => void settings.patch('highlights', { similar: patch });

  const fields: Field<FilterDef>[] = [
    { key: 'name', label: 'Name', kind: 'text', placeholder: 'Name', width: '140px' },
    { key: 'expr', label: 'Filter', kind: 'area', placeholder: 'author.subbed && message.content contains "hi"' },
  ];

  /** Why each filter doesn't read, by text. */
  let problems = $state<Record<string, string | null>>({});
  let names = $state.raw<FilterName[]>([]);
  let showNames = $state(false);

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
  const clamp = (text: string, min: number, max: number) => Math.min(max, Math.max(min, Math.round(Number(text) || min)));
</script>

<h2>Filters</h2>

<p class="intro">
  A chat shows only what passes its filters. Pick them from the chat’s menu. Same language as Chatterino.
  <button class="link" onclick={toggleNames}>{showNames ? 'Hide' : 'Show'} what you can use</button>
</p>
{#if showNames}
  <div class="names">
    {#each names as n (n.name)}<code title={n.kind}>{n.name}</code>{/each}
  </div>
{/if}
<RuleList
  items={h.filters}
  {fields}
  make={() => ({ id: id(), name: '', expr: '' })}
  onchange={(filters) => void settings.patch('highlights', { filters })}
  problem={(f) => (f.expr.trim() ? (problems[f.expr] ?? null) : null)}
  addLabel="Add filter"
/>

<h3 class="label">Near-duplicates</h3>
<Row label="Mark near-duplicates" hint="Messages much like one sent just before. Ctrl+H hides or shows them.">
  <Toggle label="Mark near-duplicates" checked={s.enabled} onchange={(v) => sim({ enabled: v })} />
</Row>
<Row label="Hide them" hint="Instead of greying them out.">
  <Toggle label="Hide them" checked={s.hide} onchange={(v) => sim({ hide: v })} />
</Row>
<Row label="How alike" hint="Percent. Higher only catches closer copies.">
  <Stepper label="How alike" value={s.threshold} min={50} max={100} unit="%" onchange={(v) => sim({ threshold: v })} />
</Row>
<Row label="Within" hint="Seconds back to compare with.">
  <input
    class="field num"
    type="number"
    aria-label="Within"
    min="1"
    max="600"
    value={s.maxAgeSecs}
    onchange={(e) => sim({ maxAgeSecs: clamp(e.currentTarget.value, 1, 600) })}
  />
</Row>
<Row label="Messages back" hint="How many earlier messages to compare with.">
  <Stepper label="Messages back" value={s.depth} min={1} max={5} onchange={(v) => sim({ depth: v })} />
</Row>
<Row label="Only from the same person">
  <Toggle label="Only from the same person" checked={s.sameUser} onchange={(v) => sim({ sameUser: v })} />
</Row>
<Row label="Check your own messages">
  <Toggle label="Check your own messages" checked={s.own} onchange={(v) => sim({ own: v })} />
</Row>

<style>
  .intro {
    margin: 0 0 8px;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .link {
    padding: 0;
    background: none;
    border: 0;
    color: var(--accent-text);
    font-weight: 600;
  }
  .names {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 10px;
  }
  .num {
    width: 80px;
  }
  code {
    padding: 1px 5px;
    background: var(--surface);
    border-radius: var(--radius);
    font-size: 0.75rem;
  }
</style>
