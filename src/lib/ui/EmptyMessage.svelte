<script lang="ts">
  import type { HTMLAttributes } from "svelte/elements";

  let { children, class: className = "", ...rest }: HTMLAttributes<HTMLDivElement> = $props();
</script>

<div {...rest} class="text-muted-foreground px-3 py-4 text-sm {className}">
  {@render children?.()}
</div>
