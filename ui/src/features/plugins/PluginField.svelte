<!-- S5 One setting a plugin asks for, drawn from its info file. -->
<script lang="ts">
  import Row from '../../components/Row.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import type { PluginField } from '../../ipc/bindings/PluginField';
  import { pluginsIpc } from '../../ipc/plugins';
  import { toast } from '../../lib/toast.svelte';
  import { errorText } from '../../lib/errors';
  import Select from '../../components/Select.svelte';

  interface Props {
    plugin: string;
    field: PluginField;
    value: unknown;
  }

  let { plugin, field, value }: Props = $props();

  function save(v: unknown) {
    pluginsIpc.setValue(plugin, field.key, v).catch((e) => toast.error(errorText(e)));
  }

  function saveText(e: Event & { currentTarget: HTMLInputElement }) {
    if (e.currentTarget.value !== value) save(e.currentTarget.value);
  }

  function saveNumber(e: Event & { currentTarget: HTMLInputElement }) {
    const n = Number(e.currentTarget.value);
    if (Number.isFinite(n) && n !== value) save(n);
  }
</script>

<Row label={field.label} hint={field.hint}>
  {#if field.type === 'toggle'}
    <Toggle label={field.label} checked={value === true} onchange={save} />
  {:else if field.type === 'text'}
    <input
      class="field"
      aria-label={field.label}
      value={String(value ?? '')}
      maxlength="500"
      onchange={saveText}
      onkeydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
    />
  {:else if field.type === 'number'}
    <input
      class="field num"
      type="number"
      aria-label={field.label}
      value={Number(value ?? 0)}
      min={field.min}
      max={field.max}
      onchange={saveNumber}
    />
  {:else}
    <Select label={field.label} width="200px" value={String(value ?? '')} options={field.options} onchange={save} />
  {/if}
</Row>

<style>
  input.field {
    width: 200px;
  }
  input.num {
    width: 100px;
  }
</style>
