<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  let query = $state("");
  let input: HTMLInputElement | undefined = $state();

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

<svelte:window on:keydown={onKeydown} on:blur={() => invoke("dismiss")} />

<main>
  <input
    bind:this={input}
    bind:value={query}
    placeholder="Search..."
    spellcheck="false"
    autocomplete="off"
  />
</main>

<style>
  :global(html, body) {
    margin: 0;
    background: transparent;
    overflow: hidden;
  }
  main {
    font-family: -apple-system, "Segoe UI", system-ui, sans-serif;
    display: flex;
    align-items: center;
    height: 100vh;
    box-sizing: border-box;
    background: rgba(30, 30, 34, 0.86);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 14px;
    padding: 0 22px;
  }
  input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    color: #f2f2f4;
    font-size: 26px;
  }
  input::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }
</style>
