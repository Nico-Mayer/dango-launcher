<script lang="ts">
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Command } from "bits-ui";
  import { onMount } from "svelte";
  import ActionPanel from "./lib/ActionPanel.svelte";
  import Icon, { namedIcon } from "./lib/Icon.svelte";
  import { pointerOwnsSelection } from "./lib/pointer.svelte";
  import ProtocolView from "./lib/ProtocolView.svelte";
  import { highlight } from "./lib/highlight";
  import { matchesShortcut, type ActionResponse, type ResultItem, type ResultsPayload } from "./lib/types";
  import type { ViewTree } from "./protocol/ViewTree";

  const PROTOCOL_VERSION = 1;

  let query = $state("");
  let results = $state<ResultItem[]>([]);
  let selectedId = $state("");
  let stack = $state<ViewTree[]>([]);
  let panelOpen = $state(false);
  let protocolError = $state(false);
  /// The extension whose command pushed what is on the stack, so an action
  /// chosen inside its view goes back to it.
  let viewOwner = $state<string | null>(null);
  /// The query inside a pushed view, kept apart from the root one so popping
  /// back does not lose what was typed at the root.
  let viewQuery = $state("");
  let viewInputEl = $state<HTMLInputElement | null>(null);
  let failure = $state<string | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);

  // The query whose results we will display; a late event for an older query is
  // dropped so cancelled results never show.
  let liveQuery = "";

  const selectedItem = $derived(
    results.find((r) => r.id === selectedId) ?? results[0],
  );

  function runSearch(q: string) {
    liveQuery = q;
    invoke("search", { query: q });
  }

  function resetToRoot() {
    query = "";
    results = [];
    stack = [];
    viewOwner = null;
    viewQuery = "";
    panelOpen = false;
    protocolError = false;
    failure = null;
    runSearch("");
  }

  async function runAction(item: ResultItem, actionId: string | undefined) {
    if (!actionId) return;
    failure = null;
    panelOpen = false;
    const response = (await invoke("run_action", {
      extensionId: item.extensionId,
      itemId: item.id,
      actionId,
    })) as ActionResponse;
    if (response.kind === "copy") {
      await navigator.clipboard.writeText(response.text);
      invoke("dismiss");
    } else if (response.kind === "failed") {
      failure = response.message;
    }
    // A plain success already hid the launcher in the backend.
  }

  /// An action chosen inside a pushed view goes to the extension that owns the
  /// running command, through the same path a root result uses.
  async function runViewAction(actionId: string, itemId: string | null) {
    if (!viewOwner) return;
    failure = null;
    const response = (await invoke("run_action", {
      extensionId: viewOwner,
      itemId: itemId ?? "",
      actionId,
    })) as ActionResponse;
    if (response.kind === "copy") {
      await navigator.clipboard.writeText(response.text);
      invoke("dismiss");
    } else if (response.kind === "failed") {
      failure = response.message;
    }
  }

  /// Confirming a result acts on it: a command is invoked, anything else runs
  /// its primary action. A command carries no actions, which is what tells the
  /// two apart.
  async function confirm(item: ResultItem | undefined) {
    if (!item) return;
    if (item.actions.length === 0) {
      failure = null;
      panelOpen = false;
      const owner = await invoke<string>("invoke_command", { commandId: item.id }).then(
        (id) => id,
        (reason) => {
          failure = String(reason);
          return null;
        },
      );
      if (owner) viewOwner = owner;
      return;
    }
    runAction(item, item.actions[0].id);
  }

  // Keys the Command primitive does not own: Escape's two-stage dismiss, the
  // action panel, and per-action shortcuts. Arrow and Enter navigation and
  // scroll-into-view are handled by Command itself.
  function onKeydown(event: KeyboardEvent) {
    if (protocolError) {
      if (event.key === "Escape") {
        event.preventDefault();
        protocolError = false;
      }
      return;
    }
    if (panelOpen) return;

    if (event.key === "Escape") {
      event.preventDefault();
      if (stack.length > 0) {
        stack = stack.slice(0, -1);
        failure = null;
        viewQuery = "";
        if (stack.length === 0) viewOwner = null;
      } else if (query.length > 0) {
        query = "";
      } else {
        invoke("dismiss");
      }
    } else if (stack.length > 0) {
      // A pushed view owns its own selection, so it owns its own action panel.
      return;
    } else if (event.key.toLowerCase() === "k" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      if (selectedItem && selectedItem.actions.length > 0) panelOpen = true;
    } else if (selectedItem) {
      for (const action of selectedItem.actions) {
        if (action.shortcut && matchesShortcut(event, action.shortcut)) {
          event.preventDefault();
          runAction(selectedItem, action.id);
          break;
        }
      }
    }
  }

  /// A named icon is handled by the icon component; anything else is a file an
  /// extension extracted, which the webview reaches through the asset protocol.
  function iconSrc(path: string | null | undefined): string | null {
    return path && !namedIcon(path) ? convertFileSrc(path) : null;
  }

  onMount(() => {
    // Runs while the window is still parked offscreen, so the first activation
    // already has the frecent items and their icons painted. Leaving it until
    // the first activation cost 40ms of icon loading and showed an empty list.
    runSearch("");
    invoke("warmup_done");
    const unlisten = [
      listen<number | null>("dango://activate", (event) => {
        inputEl?.focus();
        inputEl?.select();
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            if (event.payload !== null) invoke("report_paint", { id: event.payload });
          }),
        );
      }),
      listen("dango://reset", () => resetToRoot()),
      listen<string>("dango://failed", (event) => (failure = event.payload)),
      listen<ResultsPayload>("dango://results", (event) => {
        if (event.payload.query !== liveQuery) return;
        results = event.payload.items;
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

  // A view arriving takes the cursor, so typing narrows it without a click.
  $effect(() => {
    if (stack.length > 0) viewInputEl?.focus();
  });

  // Re-query on every keystroke; the backend cancels the previous run.
  $effect(() => {
    const q = query;
    runSearch(q);
  });
</script>

<svelte:window onkeydown={onKeydown} onblur={() => invoke("dismiss")} />

{#snippet footer(primaryLabel: string)}
  <div
    class="border-border-card text-muted-foreground flex h-10 shrink-0 items-center justify-between border-t px-4 text-xs"
  >
    <span class="text-foreground-alt font-medium">Dango</span>
    <div class="flex items-center gap-4">
      <span class="flex items-center gap-1.5">
        {primaryLabel}
        <kbd class="bg-muted rounded px-1.5 py-0.5 font-sans">↵</kbd>
      </span>
      <span class="flex items-center gap-1.5">
        Actions
        <kbd class="bg-muted rounded px-1.5 py-0.5 font-sans">Ctrl K</kbd>
      </span>
    </div>
  </div>
{/snippet}

{#if protocolError}
  <main
    class="border-border-card bg-background flex h-screen w-screen flex-col justify-center gap-2 overflow-hidden rounded-[14px] border px-6"
  >
    <span class="text-foreground text-sm">This view needs a newer version of Dango.</span>
    <span class="text-muted-foreground text-xs">Press Escape to go back.</span>
  </main>
{:else if stack.length > 0}
  <main
    class="border-border-card bg-background flex h-screen w-screen flex-col overflow-hidden rounded-[14px] border"
  >
    <input
      bind:this={viewInputEl}
      bind:value={viewQuery}
      placeholder="Search..."
      spellcheck={false}
      autocomplete="off"
      class="text-foreground placeholder:text-muted-foreground h-16 w-full shrink-0 bg-transparent px-5 text-2xl focus:outline-none"
    />
    <div class="border-border-card min-h-0 flex-1 overflow-y-auto border-t">
      <ProtocolView
        tree={stack[stack.length - 1]}
        query={viewQuery}
        onaction={runViewAction}
        onsubmit={() => {}}
      />
    </div>
    {#if failure}
      <div
        class="text-destructive border-border-card flex shrink-0 items-center gap-2 border-t px-5 py-2 text-sm"
      >
        <Icon name="circle-alert" size={16} />
        {failure}
      </div>
    {/if}
    {@render footer("Select")}
  </main>
{:else}
  <Command.Root
    shouldFilter={false}
    disablePointerSelection={!pointerOwnsSelection()}
    bind:value={selectedId}
    class="border-border-card bg-background flex h-screen w-screen flex-col overflow-hidden rounded-[14px] border"
  >
    <Command.Input
      bind:ref={inputEl}
      bind:value={query}
      placeholder="Search for apps and commands..."
      spellcheck={false}
      autocomplete="off"
      class="text-foreground placeholder:text-muted-foreground h-16 w-full shrink-0 bg-transparent px-5 text-2xl focus:outline-none"
    />
    <Command.List class="border-border-card min-h-0 flex-1 overflow-y-auto border-t">
      <Command.Viewport class="p-2">
        {#each results as item (item.id)}
          <Command.Item
            value={item.id}
            onSelect={() => confirm(item)}
            class="flex h-12 items-center gap-3 rounded-lg px-3 data-[selected]:bg-muted"
          >
            {#if namedIcon(item.icon)}
              <div class="text-foreground-alt flex h-7 w-7 shrink-0 items-center justify-center">
                <Icon name={namedIcon(item.icon)!} size={20} />
              </div>
            {:else if iconSrc(item.icon)}
              <img src={iconSrc(item.icon)} alt="" class="h-7 w-7 shrink-0" />
            {:else}
              <div class="bg-muted h-7 w-7 shrink-0 rounded"></div>
            {/if}
            <div class="flex min-w-0 items-baseline gap-2">
              <span class="text-foreground truncate text-sm">
                {#each highlight(item.title, item.matchPositions) as seg}
                  <span class={seg.matched ? "font-semibold" : ""}>{seg.text}</span>
                {/each}
              </span>
              {#if item.subtitle}
                <span class="text-muted-foreground truncate text-xs">{item.subtitle}</span>
              {/if}
            </div>
          </Command.Item>
        {/each}
        {#if results.length === 0 && query.length > 0}
          <div class="text-muted-foreground px-3 py-4 text-sm">No results</div>
        {/if}
      </Command.Viewport>
    </Command.List>

    {#if failure}
      <div
        class="text-destructive border-border-card flex shrink-0 items-center gap-2 border-t px-5 py-2 text-sm"
      >
        <Icon name="circle-alert" size={16} />
        {failure}
      </div>
    {/if}

    {@render footer("Open")}

    {#if panelOpen && selectedItem}
      <ActionPanel
        actions={selectedItem.actions}
        onrun={(id) => runAction(selectedItem, id)}
        onclose={() => (panelOpen = false)}
      />
    {/if}
  </Command.Root>
{/if}
