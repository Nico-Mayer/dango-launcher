<script lang="ts">
  import type { HTMLAttributes } from "svelte/elements";

  let { children, class: className = "", ...rest }: HTMLAttributes<HTMLElement> = $props();
</script>

<main {...rest} class="border-border-card bg-background flex h-screen w-screen flex-col overflow-hidden rounded-card border-edge {className}">
  {@render children?.()}
</main>
