<!-- V108-F08 Text with its emoji drawn as Twemoji pictures (flags don't exist in Segoe UI Emoji).
     The characters stay as alt text, so copying still gives them. -->
<script lang="ts">
  import EmojiImg from '../features/chat/EmojiImg.svelte';
  import { splitEmoji } from '../features/emotes/emoji/split';

  let { text }: { text: string } = $props();
</script>

{#each splitEmoji(text) as p, i (i)}{#if p.emoji}<span class="e"><EmojiImg char={p.text} /></span>{:else}{p.text}{/if}{/each}

<style>
  .e :global(.emoji-img) {
    height: 1.2em;
    vertical-align: -0.2em;
  }
</style>
