<script lang="ts">
  import { Checkbox, type WithoutChildrenOrChild } from "bits-ui";
  import Icon from "../Icon.svelte";

  let { checked = $bindable(false), ref = $bindable(null), class: className = "", ...rest }: WithoutChildrenOrChild<Checkbox.RootProps> = $props();
</script>

<Checkbox.Root {...rest} bind:checked bind:ref class="accent-control-accent border-border-input bg-control-bg text-control-text data-[state=checked]:bg-control-accent data-[state=checked]:text-background focus-visible:outline-control-focus disabled:text-text-disabled flex size-icon-status shrink-0 items-center justify-center self-start rounded-keycap border-edge {className}">
  {#snippet children({ checked })}
    {#if checked}<Icon name="check" size={12} class="size-icon-small" />{/if}
  {/snippet}
</Checkbox.Root>
