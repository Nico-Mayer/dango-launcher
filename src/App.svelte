<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Command } from "bits-ui";
  import { onMount } from "svelte";

  let query = $state("");
  let input = $state<HTMLInputElement | null>(null);

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape") return;
    event.preventDefault();
    if (query.length > 0) {
      query = "";
    } else {
      invoke("dismiss");
    }
  }

  onMount(() => {
    requestAnimationFrame(() => requestAnimationFrame(() => invoke("warmup_done")));

    const activate = listen<number | null>("dango://activate", (event) => {
      input?.focus();
      // Two frames approximates "after paint"; a single frame fires before it.
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          if (event.payload !== null) invoke("report_paint", { id: event.payload });
        }),
      );
    });
    const reset = listen("dango://reset", () => {
      query = "";
    });

    return () => {
      activate.then((un) => un());
      reset.then((un) => un());
    };
  });
</script>

<svelte:window onkeydown={onKeydown} onblur={() => invoke("dismiss")} />

<Command.Root
  class="border-border-card bg-background/85 flex h-screen w-screen flex-col overflow-hidden rounded-[14px] border backdrop-blur-xl"
>
  <Command.Input
    bind:ref={input}
    bind:value={query}
    placeholder="Search..."
    spellcheck={false}
    autocomplete="off"
    class="text-foreground placeholder:text-muted-foreground h-full w-full bg-transparent px-6 text-[26px] focus:outline-none"
  />
</Command.Root>
