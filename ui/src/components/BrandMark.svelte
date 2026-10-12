<!-- The cyex mark (3.12), drawn like the cyex logs header: three square bars,
     the middle one in the accent. -->
<script lang="ts">
  // No props: the mark always uses the current --text and --accent.
</script>

<span class="mark" role="img" aria-label="cyex deck"><i></i><i></i><i></i></span>

<style>
  .mark {
    position: relative;
    display: block;
    width: 25px;
    height: 22px;
    flex: none;
  }
  i {
    position: absolute;
    left: 0;
    width: 15px;
    height: 6px;
    background: var(--text);
  }
  i:nth-child(1) {
    top: 0;
  }
  i:nth-child(2) {
    top: 8px;
    left: 7px;
    width: 18px;
    background: var(--accent);
  }
  i:nth-child(3) {
    top: 16px;
  }
</style>
