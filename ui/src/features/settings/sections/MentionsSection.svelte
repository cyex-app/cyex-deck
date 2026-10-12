<!-- Mentions (V104-01): your name and your own words, each with its own options, in every
     open chat. Global mentions (V104-02) look through every chat logs.cyex.app keeps; the
     list is matched on this PC and never sent anywhere. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { GlobalStatus } from '../../../ipc/bindings/GlobalStatus';
  import type { Mentions } from '../../../ipc/bindings/Mentions';
  import type { YourName } from '../../../ipc/bindings/YourName';
  import { mentionsIpc } from '../../../ipc/mentions';
  import { settings } from '../../../lib/settings.svelte';
  import { mentionsTab } from '../../chat/mentionsTab.svelte';
  import MentionList from '../mentions/MentionList.svelte';
  import MentionRow from '../mentions/MentionRow.svelte';
  import { statusText } from '../mentions/status';

  const m = $derived(settings.v.mentions);
  const save = (patch: Partial<Mentions>) => void settings.patch('mentions', patch);

  let status = $state<GlobalStatus>({ state: 'off' });
  $effect(() => {
    void mentionsIpc.globalStatus().then((s) => (status = s));
    const off = mentionsIpc.onGlobalStatus((s) => (status = s));
    return () => void off.then((f) => f());
  });
  const hasGlobal = $derived(m.globalList.some((x) => x.enabled && x.pattern.trim()));
</script>

<h2>Mentions</h2>

<h3 class="label">Your name</h3>
<MentionRow
  setting="mentions.me.enabled"
  title="Your name and replies to you"
  item={m.me}
  onchange={(me) => save({ me: me as YourName })}
/>

<h3 class="label">Your words</h3>
<MentionList
  setting="mentions.list"
  items={m.list}
  onchange={(list) => save({ list })}
  empty="Words that ping you in any open chat."
/>
<div class="tab">
  <Button small icon="list" onclick={() => mentionsTab.open()}>Open the Mentions tab</Button>
</div>

<h3 class="label">Global mentions</h3>
<Row setting="mentions.global" label="Global mentions" hint="From logs.cyex.app: every chat it logs. Uses about 0,8 GB an hour while on. Your list stays on this PC.">
  <Toggle label="Global mentions" checked={m.global} onchange={(v) => save({ global: v })} />
</Row>
{#if m.global}
  <p class="status" class:live={status.state === 'connected'}>
    <span class="dot"></span>{statusText(status, hasGlobal)}
  </p>
  <MentionList
    setting="mentions.globalList"
    items={m.globalList}
    onchange={(globalList) => save({ globalList })}
    empty="Words to find in any chat, open or not."
  />
{/if}

<style>
  .tab {
    padding: 4px 0 12px;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 10px 0 2px;
    color: var(--muted);
    font-size: 0.82rem;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--dim);
  }
  .live .dot {
    background: var(--green);
  }
</style>
