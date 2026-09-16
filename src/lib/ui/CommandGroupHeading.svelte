<script lang="ts">
  import { Command } from "bits-ui";

  let { ref = $bindable(null), class: className = "", ...rest }: Command.GroupHeadingProps = $props();
</script>

<Command.GroupHeading {...rest} bind:ref class="text-muted-foreground px-3 pt-2 pb-1 text-xs font-medium {className}" />
