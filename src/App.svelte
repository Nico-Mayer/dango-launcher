<script lang="ts">
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import ActionPanel from "./lib/ActionPanel.svelte";
  import ProtocolView from "./lib/ProtocolView.svelte";
  import { highlight } from "./lib/highlight";
  import { matchesShortcut, type ActionResponse, type ResultItem, type ResultsPayload } from "./lib/types";
  import type { ViewTree } from "./protocol/ViewTree";

  const PROTOCOL_VERSION = 1;

  let query = $state("");
  let results = $state<ResultItem[]>([]);
  let selected = $state(0);
  let stack = $state<ViewTree[]>([]);
  let panelOpen = $state(false);
  let protocolError = $state(false);
  let failure = $state<string | null>(null);
  let input = $state<HTMLInputElement | null>(null);
  let surface = $state<HTMLElement | null>(null);

  // The query whose results we are willing to display; a late event for an
  // older query is dropped.
  let liveQuery = "";

  const selectedItem = $derived<ResultItem | undefined>(results[selected]);
  const atRoot = $derived(stack.length === 0 && !protocolError);

  function runSearch(q: string) {
    liveQuery = q;
    invoke("search", { query: q });
  }

  function resetToRoot() {
    query = "";
    results = [];
    selected = 0;
    stack = [];
    panelOpen = false;
    protocolError = false;
    failure = null;
    runSearch("");
  }

  async function runAction(itemId: string, actionId: string) {
    failure = null;
    const response = (await invoke("run_action", { itemId, actionId })) as ActionResponse;
    if (response.kind === "copy") {
      await navigator.clipboard.writeText(response.text);
      invoke("dismiss");
    } else if (response.kind === "failed") {
      failure = response.message;
    }
    // launched and revealed already hid the launcher in the backend.
    panelOpen = false;
  }

  function activatePrimary() {
    const item = selectedItem;
    if (item && item.actions.length > 0) {
      runAction(item.id, item.actions[0].id);
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (protocolError) {
      if (event.key === "Escape") {
        event.preventDefault();
        protocolError = false;
      }
      return;
    }
    // The action panel and pushed views handle their own keys.
    if (panelOpen || stack.length > 0) {
      if (event.key === "Escape" && stack.length > 0 && !panelOpen) {
        event.preventDefault();
        stack = stack.slice(0, -1);
      }
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      if (query.length > 0) {
        query = "";
      } else {
        invoke("dismiss");
      }
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      selected = Math.min(selected + 1, results.length - 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selected = Math.max(selected - 1, 0);
    } else if (event.key === "Enter") {
      event.preventDefault();
      activatePrimary();
    } else if (event.key.toLowerCase() === "k" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      if (selectedItem && selectedItem.actions.length > 0) panelOpen = true;
    } else if (selectedItem) {
      for (const action of selectedItem.actions) {
        if (action.shortcut && matchesShortcut(event, action.shortcut)) {
          event.preventDefault();
          runAction(selectedItem.id, action.id);
          break;
        }
      }
    }
  }

  function iconSrc(path: string | null | undefined): string | null {
    return path ? convertFileSrc(path) : null;
  }

  onMount(() => {
    invoke("warmup_done");

    const unlisten = [
      listen<number | null>("dango://activate", (event) => {
        input?.focus();
        input?.select();
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            if (event.payload !== null) invoke("report_paint", { id: event.payload });
          }),
        );
      }),
      listen("dango://reset", () => resetToRoot()),
      listen<ResultsPayload>("dango://results", (event) => {
        if (event.payload.query !== liveQuery) return;
        results = event.payload.items;
        if (selected >= results.length) selected = Math.max(0, results.length - 1);
      }),
      listen<ViewTree>("dango://render", (event) => {
        if (event.payload.protocolVersion !== PROTOCOL_VERSION) {
          protocolError = true;
          return;
        }
        stack = [...stack, event.payload];
      }),
    ];

    return () => unlisten.forEach((p) => p.then((un) => un()));
  });

  // Re-query whenever the text changes; the backend cancels the previous run.
  $effect(() => {
    const q = query;
    runSearch(q);
  });

  // Grow and shrink the window to fit its content, so the transparent surface is
  // never larger than what is drawn.
  $effect(() => {
    void results.length;
    void stack.length;
    void failure;
    void protocolError;
    void query;
    requestAnimationFrame(() => {
      if (!surface) return;
      getCurrentWindow()
        .setSize(new LogicalSize(720, surface.offsetHeight))
        .catch(() => {});
    });
  });
</script>

<svelte:window onkeydown={onKeydown} onblur={() => invoke("dismiss")} />

<main
  bind:this={surface}
  class="border-border-card bg-background/85 flex w-screen flex-col overflow-hidden rounded-[14px] border backdrop-blur-xl"
>
  {#if protocolError}
    <div class="flex flex-col gap-2 px-6 py-5">
      <span class="text-foreground text-sm">This view needs a newer version of Dango.</span>
      <span class="text-muted-foreground text-xs">Press Escape to go back.</span>
    </div>
  {:else if stack.length > 0}
    <ProtocolView
      tree={stack[stack.length - 1]}
      onaction={() => {}}
      onsubmit={() => {}}
    />
  {:else}
    <input
      bind:this={input}
      bind:value={query}
      placeholder="Search..."
      spellcheck="false"
      autocomplete="off"
      class="text-foreground placeholder:text-muted-foreground h-16 w-full shrink-0 bg-transparent px-6 text-[26px] focus:outline-none"
    />
    {#if failure}
      <div class="text-destructive border-border-card border-t px-6 py-2 text-sm">{failure}</div>
    {/if}
    {#if results.length > 0}
      <ul class="border-border-card max-h-[420px] overflow-y-auto border-t py-1">
        {#each results as item, i (item.id)}
          <li>
            <button
              type="button"
              class="flex w-full items-center gap-3 px-4 py-2 text-left {i === selected
                ? 'bg-muted'
                : ''}"
              onmouseenter={() => (selected = i)}
              onclick={() => activatePrimary()}
            >
              {#if iconSrc(item.icon)}
                <img src={iconSrc(item.icon)} alt="" class="h-8 w-8 shrink-0" />
              {:else}
                <div class="bg-muted h-8 w-8 shrink-0 rounded"></div>
              {/if}
              <div class="flex min-w-0 flex-col">
                <span class="text-foreground truncate text-sm">
                  {#each highlight(item.title, item.matchPositions) as seg}
                    <span class={seg.matched ? "text-foreground font-semibold" : ""}>{seg.text}</span>
                  {/each}
                </span>
                {#if item.subtitle}
                  <span class="text-muted-foreground truncate text-xs">{item.subtitle}</span>
                {/if}
              </div>
            </button>
          </li>
        {/each}
      </ul>
    {:else if query.length > 0}
      <div class="text-muted-foreground border-border-card border-t px-6 py-4 text-sm">
        No results
      </div>
    {/if}
  {/if}

  {#if panelOpen && selectedItem}
    <ActionPanel
      actions={selectedItem.actions}
      onrun={(id) => runAction(selectedItem.id, id)}
      onclose={() => (panelOpen = false)}
    />
  {/if}
</main>
