<script lang="ts">
  import { Command } from "bits-ui";

  let { ref = $bindable(null), class: className = "", ...rest }: Command.ViewportProps = $props();
</script>

<Command.Viewport {...rest} bind:ref class="px-2 {className}" />
