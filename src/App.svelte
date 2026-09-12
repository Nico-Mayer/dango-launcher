<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Command } from "bits-ui";
  import { onMount } from "svelte";
  import ActionPanel from "./lib/ActionPanel.svelte";
  import Icon from "./lib/Icon.svelte";
  import { pointerOwnsSelection } from "./lib/pointer.svelte";
  import ProtocolView from "./lib/ProtocolView.svelte";
  import ResultRow from "./lib/ResultRow.svelte";
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
  let failure = $state<string | null>(null);
  // A view command has been invoked and its first tree has not arrived yet.
  let working = $state(false);
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
    panelOpen = false;
    protocolError = false;
    failure = null;
    working = false;
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
      values: null,
    })) as ActionResponse;
    if (response.kind === "copy") {
      await navigator.clipboard.writeText(response.text);
      invoke("dismiss");
    } else if (response.kind === "replaced") {
      // A root action that produces a view opens it. A snippet needing
      // arguments has to ask before it can do anything.
      viewOwner = item.extensionId;
      stack = [response.tree];
    } else if (response.kind === "failed") {
      failure = response.message;
    }
    // A plain success already hid the launcher in the backend.
  }

  /// An action chosen inside a pushed view goes to the extension that owns the
  /// running command, through the same path a root result uses.
  async function runViewAction(
    actionId: string,
    itemId: string | null,
    values?: Record<string, string>,
  ) {
    if (!viewOwner) return;
    failure = null;
    const response = (await invoke("run_action", {
      extensionId: viewOwner,
      itemId: itemId ?? "",
      actionId,
      values: values ?? null,
    })) as ActionResponse;
    if (response.kind === "copy") {
      await navigator.clipboard.writeText(response.text);
      invoke("dismiss");
    } else if (response.kind === "replaced") {
      stack = [...stack.slice(0, -1), response.tree];
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
      working = true;
      const owner = await invoke<string>("invoke_command", { commandId: item.id }).then(
        (id) => id,
        (reason) => {
          failure = String(reason);
          working = false;
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
      listen<string>("dango://failed", (event) => {
        failure = event.payload;
        working = false;
      }),
      listen<ResultsPayload>("dango://results", (event) => {
        if (event.payload.query !== liveQuery) return;
        results = event.payload.items;
      }),
      listen<ViewTree>("dango://render", (event) => {
        working = false;
        if (event.payload.protocolVersion !== PROTOCOL_VERSION) {
          protocolError = true;
          return;
        }
        stack = [...stack, event.payload];
      }),
    ];
    return () => unlisten.forEach((p) => p.then((un) => un()));
  });

  /// Who owns the cursor, in one place.
  ///
  /// The root input holds it unless something else is on screen that does: a
  /// pushed view focuses its own input, the action panel and the protocol error
  /// own the keyboard while they are up. Every one of those going away has to
  /// hand the cursor back, or the launcher is left unable to type, which is how
  /// popping a view used to strand it on the body.
  $effect(() => {
    if (stack.length > 0 || panelOpen || protocolError) return;
    inputEl?.focus();
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
    <ProtocolView tree={stack[stack.length - 1]} onaction={runViewAction} />
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
            class="data-[selected]:bg-muted flex h-14 items-center gap-3 rounded-lg px-3"
          >
            <ResultRow
              title={item.title}
              subtitle={item.subtitle}
              icon={item.icon}
              matchPositions={item.matchPositions}
            />
          </Command.Item>
        {/each}
        {#if results.length === 0 && query.length > 0}
          <div class="text-muted-foreground px-3 py-4 text-sm">No results</div>
        {/if}
      </Command.Viewport>
    </Command.List>

    {#if working}
      <div
        class="text-muted-foreground border-border-card flex shrink-0 items-center gap-2 border-t px-5 py-2 text-sm"
      >
        <Icon name="loader-circle" size={16} class="animate-spin" />
        Working...
      </div>
    {/if}
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
