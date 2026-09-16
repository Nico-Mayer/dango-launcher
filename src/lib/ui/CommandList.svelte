<script lang="ts">
  import { Command } from "bits-ui";

  let { ref = $bindable(null), class: className = "", ...rest }: Command.ListProps = $props();

  // Bits wraps each item in a `display: contents` node carrying the same value,
  // so its "first item of a group" branch matches every row and scrolls only the
  // group heading, or nothing in an ungrouped list. The selected row itself is
  // left just outside the scroller, so keep it in view here.
  $effect(() => {
    const list = ref;
    if (!list) return;
    const observer = new MutationObserver(() => {
      const selected = list.querySelector<HTMLElement>("[data-command-item][data-selected]");
      if (!selected) return;
      if (selected === list.querySelector("[data-command-item]")) list.scrollTop = 0;
      else selected.scrollIntoView({ block: "nearest" });
    });
    observer.observe(list, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-selected"] });
    return () => observer.disconnect();
  });
</script>

<Command.List {...rest} bind:ref class="min-h-0 flex-1 overflow-y-auto {className}" />
