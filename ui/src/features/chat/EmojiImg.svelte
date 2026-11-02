<!-- V108-C30 One emoji as a Twemoji picture. The character stays as alt text, so copying still gives
     it; a picture that doesn't load falls back to the character. -->
<script lang="ts">
  import { twemojiUrl } from '../emotes/emoji/twemoji';

  let { char }: { char: string } = $props();
  /** The picture that failed; another emoji in a reused instance gets its own try. */
  let failedSrc = $state<string | null>(null);
  const src = $derived(twemojiUrl(char));
  const failed = $derived(failedSrc === src);
</script>

{#if src && !failed}<img class="emoji-img" {src} alt={char} loading="lazy" decoding="async" draggable="false" onerror={() => (failedSrc = src)} />{:else}{char}{/if}
