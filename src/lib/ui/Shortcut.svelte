<script lang="ts">
  import type { HTMLAttributes } from "svelte/elements";

  let { children, class: className = "", ...rest }: HTMLAttributes<HTMLElement> = $props();
</script>

<kbd {...rest} class="shrink-0 whitespace-nowrap bg-keycap-bg text-keycap-text rounded-keycap px-1.5 py-0.5 font-sans {className}">{@render children?.()}</kbd>
