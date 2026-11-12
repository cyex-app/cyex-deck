<!-- V110-N6 Your own menu entries and usercard buttons, on the Menus page (they were on Commands). -->
<script lang="ts">
  import type { CommandRow } from '../../../ipc/bindings/CommandRow';
  import type { Commands } from '../../../ipc/bindings/Commands';
  import { settings } from '../../../lib/settings.svelte';
  import { GUIDE } from '../commands/guide';
  import GuideText from '../commands/GuideText.svelte';
  import { hiddenBySubmenu, type MenuView, menuProblem, menuRow, menuView } from '../commands/rows';
  import type { Field } from '../rules/fields';
  import RuleList from '../rules/RuleList.svelte';
  import { CAPS } from '../rules/caps';
  import SectionHead from '../SectionHead.svelte';

  const c = $derived(settings.v.commands);
  const save = (patch: Partial<Commands>) => void settings.patch('commands', patch);

  const menuFields: Field<MenuView>[] = [
    { key: 'enabled', label: 'On', kind: 'check' },
    { key: 'label', label: 'Label', kind: 'text', placeholder: 'Label', width: '150px', lead: true },
    { key: 'command', label: 'Command', kind: 'text', placeholder: 'Command', main: true, mono: true },
  ];

  type MenuKey = 'userMenu' | 'messageMenu' | 'channelMenu' | 'usercardButtons';
  const MENUS: { key: MenuKey; label: string; hint: string }[] = [
    { key: 'userMenu', label: 'Name menu', hint: 'Right-click a name.' },
    { key: 'messageMenu', label: 'Message menu', hint: 'Right-click a message.' },
    { key: 'channelMenu', label: 'Chat menu', hint: 'The ⋯ button in a chat’s header.' },
  ];
</script>

{#snippet menuList(key: MenuKey, add: string, gone: string)}
  {@const rows: CommandRow[] = c[key]}
  <RuleList
    items={rows.map(menuView)}
    max={CAPS.commandLines}
    fields={menuFields}
    layout="split"
    make={() => ({ enabled: true, label: '', command: '', raw: '' })}
    onchange={(v) => save({ [key]: v.map(menuRow) })}
    addLabel={add}
    deleted={gone}
    empty="Nothing added."
    emptyIcon="list"
    reorder={{ key: `commands.${key}`, label: (v, i) => v.label || `Row ${i + 1}` }}
    problem={(v) => menuProblem(v)}
    note={(_, i) => (hiddenBySubmenu(rows, i) ? 'Hidden: the submenu above is off.' : null)}
  />
{/snippet}

<SectionHead title={GUIDE.menus.title}><GuideText entry={GUIDE.menus} /></SectionHead>
{#each MENUS as m (m.key)}
  <div class="sub" data-setting="commands.{m.key}">
    <span class="sub-title">{m.label}</span><span class="sub-hint">{m.hint}</span>
  </div>
  {@render menuList(m.key, 'Add entry', 'Entry deleted.')}
{/each}

<SectionHead title={GUIDE.usercard.title} setting="commands.usercardButtons"><GuideText entry={GUIDE.usercard} /></SectionHead>
{@render menuList('usercardButtons', 'Add button', 'Button deleted.')}

<style>
  .sub {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 14px 0 0;
    font-size: var(--text-sub);
  }
  .sub-title {
    font-weight: 600;
  }
  .sub-hint {
    color: var(--dim);
    font-size: var(--text-hint);
  }
</style>
