<script lang="ts">
  import type { HTMLInputAttributes, HTMLTextareaAttributes } from "svelte/elements";

  let { multiline = false, value = $bindable(""), type = "text", class: className = "", ...rest }: Omit<HTMLInputAttributes & HTMLTextareaAttributes, "value" | "type"> & {
    multiline?: boolean;
    value?: string;
    type?: "text" | "password";
  } = $props();
  const controlClass = "border-border-input bg-control-bg text-control-text focus:border-focus-ring disabled:text-text-disabled rounded-field border-edge px-field-inset py-2 text-sm outline-none";
</script>

{#if multiline}
  <textarea {...rest} spellcheck={false} bind:value class="{controlClass} resize-none font-mono {className}"></textarea>
{:else}
  <input {...rest} {type} spellcheck={false} bind:value class="{controlClass} {className}" />
{/if}
