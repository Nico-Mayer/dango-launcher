<script lang="ts">
  import { Command } from "bits-ui";

  let { ref = $bindable(null), value = $bindable(""), class: className = "", ...rest }: Command.InputProps = $props();
</script>

<Command.Input spellcheck={false} autocomplete="off" {...rest} bind:ref bind:value class="text-foreground placeholder:text-muted-foreground h-search w-full shrink-0 bg-transparent px-5 text-2xl outline-none {className}" />
