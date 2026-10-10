<script lang="ts">
  import type { HTMLAttributes } from "svelte/elements";

  let { children, development = false, class: className = "", ...rest }: HTMLAttributes<HTMLElement> & { development?: boolean } = $props();
</script>

<main {...rest} data-development={development ? "" : undefined} class="border-border-card bg-background flex h-screen w-screen flex-col overflow-hidden rounded-card border-edge data-development:border-dev-edge data-development:inset-shadow-dev-edge {className}">
  {@render children?.()}
</main>
