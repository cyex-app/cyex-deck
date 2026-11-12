<!-- V110-N13 `/poll` and `/prediction`: the card for the chat's channel in a floating panel
     (`votePanels.svelte.ts` opens and closes it). -->
<script lang="ts">
  import FloatPanel from '../../components/FloatPanel.svelte';
  import PollCard from './PollCard.svelte';
  import PredictionCard from './PredictionCard.svelte';
  import type { VoteKind } from './voteGate';

  interface Props {
    kind: VoteKind;
    login: string;
    account: string;
    question: string;
    nth: number;
    onclose: () => void;
  }

  let { kind, login, account, question, nth, onclose }: Props = $props();

  let panel = $state<FloatPanel>();
  export const focus = () => panel?.focus();
</script>

<FloatPanel bind:this={panel} label={kind === 'poll' ? `Poll in ${login}` : `Prediction in ${login}`} {nth} {onclose}>
  {#if kind === 'poll'}
    <PollCard {login} {account} {question} autofocus />
  {:else}
    <PredictionCard {login} {account} {question} autofocus />
  {/if}
</FloatPanel>
