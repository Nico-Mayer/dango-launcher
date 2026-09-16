<script lang="ts">
  import { Command } from "bits-ui";

  let { ref = $bindable(null), class: className = "", ...rest }: Command.ItemProps = $props();
</script>

<Command.Item {...rest} bind:ref class="data-[selected]:bg-selection-bg data-[selected]:inset-shadow-selection [[data-pointer]_&:hover:not([data-selected])]:bg-hover-bg flex h-result items-center gap-3 rounded-lg px-row-inset {className}" />
