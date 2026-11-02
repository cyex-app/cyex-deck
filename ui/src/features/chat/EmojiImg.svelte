<!-- V108-C30 One emoji as a Twemoji picture. The character stays as alt text, so copying still gives
     it; a picture that doesn't load falls back to the character. -->
<script lang="ts">
  import { twemojiUrl } from '../emotes/emoji/twemoji';

  let { char }: { char: string } = $props();
  let failed = $state(false);
  const src = $derived(twemojiUrl(char));
</script>

{#if src && !failed}<img class="emoji-img" {src} alt={char} loading="lazy" decoding="async" draggable="false" onerror={() => (failed = true)} />{:else}{char}{/if}
