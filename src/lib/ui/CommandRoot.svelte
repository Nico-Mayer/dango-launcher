<script lang="ts">
  import { Command } from "bits-ui";

  let { ref = $bindable(null), value = $bindable(""), class: className = "", ...rest }: Command.RootProps = $props();
</script>

<Command.Root shouldFilter={false} disablePointerSelection vimBindings={false} {...rest} bind:ref bind:value class="flex min-h-0 flex-1 flex-col {className}" />
