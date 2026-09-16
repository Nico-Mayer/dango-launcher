<script lang="ts">
  import { Label } from "bits-ui";
  import type { Snippet } from "svelte";
  import Icon from "../Icon.svelte";

  let { label, help, helpEmphasized = false, error, children }: {
    label: string;
    help?: string;
    helpEmphasized?: boolean;
    error?: string | null;
    children: Snippet<[{ id: string; "aria-describedby": string | undefined; "aria-invalid": boolean }]>;
  } = $props();
  const id = $props.id();
</script>

<div class="flex min-w-0 flex-col gap-1.5 wrap-anywhere">
  <Label.Root for={id} class="text-foreground-alt text-xs font-medium">{label}</Label.Root>
  {@render children({ id, "aria-describedby": error || help ? `${id}-description` : undefined, "aria-invalid": !!error })}
  {#if error}
    <span id={`${id}-description`} class="text-destructive flex items-center gap-1.5 text-xs">
      <Icon name="circle-alert" size={12} class="size-icon-small" />
      {error}
    </span>
  {:else if help}
    <span id={`${id}-description`} class="text-xs {helpEmphasized ? 'text-foreground-alt' : 'text-muted-foreground'}">{help}</span>
  {/if}
</div>
