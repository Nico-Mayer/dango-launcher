<script lang="ts">
  import { Command } from "bits-ui";
  import ActionPanel from "../../src/lib/ActionPanel.svelte";
  import { longActions } from "./data";
  let input: HTMLInputElement | null = $state(null);
  let value = $state("parent-1");
  let open = $state(false);
  let runs = $state(0);
  let parentKeys = $state(0);
</script>

<Command.Root bind:value shouldFilter={false} disablePointerSelection vimBindings={false} onkeydown={() => parentKeys++} class="bg-background text-foreground flex h-[400px] flex-col">
  <Command.Input bind:ref={input} aria-label="Search" />
  <Command.List><Command.Viewport>
    <Command.Item value="parent-1">Parent one</Command.Item>
    <Command.Item value="parent-2">Parent two</Command.Item>
  </Command.Viewport></Command.List>
  <footer class="mt-auto flex justify-end p-2">
    <ActionPanel actions={longActions} bind:open returnFocus={() => input} onrun={() => runs++} />
  </footer>
</Command.Root>
<output data-runs={runs} data-parent-keys={parentKeys}></output>
