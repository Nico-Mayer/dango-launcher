<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Command } from "bits-ui";
  import { onMount } from "svelte";
  import ActionPanel from "./lib/ActionPanel.svelte";
  import Icon from "./lib/Icon.svelte";
  import { inUserGesture, pointerActive } from "./lib/input.svelte";
  import { modKey } from "./lib/platform";
  import ProtocolView from "./lib/ProtocolView.svelte";
  import ResultRow from "./lib/ResultRow.svelte";
  import {
    matchesShortcut,
    type ActionResponse,
    type RenderPayload,
    type ResultItem,
    type ResultsPayload,
  } from "./lib/types";
  import type { ViewTree } from "./protocol/ViewTree";

  const PROTOCOL_VERSION = 1;

  let query = $state("");
  let results = $state<ResultItem[]>([]);
  /// What the keyboard or a click last picked. The selection is derived from
  /// it rather than stored, so results arriving without the picked item fall
  /// back to the first row instead of leaving the list with nothing selected.
  let pickedId = $state("");
  let stack = $state<ViewTree[]>([]);
  let panelOpen = $state(false);
  let protocolError = $state(false);
  /// The extension whose command pushed what is on the stack, so an action
  /// chosen inside its view goes back to it.
  let viewOwner = $state<string | null>(null);
  /// The run whose tree is on top of the stack. A streaming command replaces
  /// its own view as the answer grows, so a tree from the run already showing
  /// takes the place of the one there rather than pushing another.
  let shownInvocation: number | null = null;
  let failure = $state<string | null>(null);
  // The title of a command that has been invoked and has neither finished nor
  // shown a view yet.
  let workingTitle = $state<string | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLElement | null>(null);

  // The query whose results we will display; a late event for an older query is
  // dropped so cancelled results never show.
  let liveQuery = "";

  /// The same idea for views. Cancelling a streaming command does not recall the
  /// tree it has already emitted, so one more can land after the user has left:
  /// with nothing on the stack to replace it goes on as a new view, and the
  /// answer they walked away from is back on screen, frozen part-written.
  /// Invocation ids only ever go up, so one high-water mark covers every run at
  /// or before the one abandoned.
  let lastInvocation = -1;
  let abandonedInvocation = -1;

  /// Leaves whatever is showing, and with it anything still arriving for it.
  function abandonShownView() {
    abandonedInvocation = lastInvocation;
    shownInvocation = null;
  }

  const selectedId = $derived(
    results.some((r) => r.id === pickedId) ? pickedId : (results[0]?.id ?? ""),
  );
  const selectedItem = $derived(results.find((r) => r.id === selectedId));

  /// The primitive scrolls the selection into view for every row but the first:
  /// for that one it scrolls the enclosing group's heading instead, and a list
  /// without groups has none, so it returns having scrolled nothing. Arrowing
  /// back to the top then leaves the first row selected just above the fold.
  $effect(() => {
    if (selectedId && selectedId === results[0]?.id && listEl) listEl.scrollTop = 0;
  });

  function runSearch(q: string) {
    liveQuery = q;
    invoke("search", { query: q });
  }

  /// A failure deliberately survives this.
  ///
  /// An action that pastes hides the launcher before it does the work, so it
  /// can only report a failure once the window is already gone, and whether
  /// that report arrives before or after this reset is a race. Clearing it here
  /// lost the reason perhaps half the time, which is how a broken paste came to
  /// look like nothing happening at all. The message is carried to the next
  /// activation instead, and cleared as soon as the user types, acts, or
  /// presses Escape.
  function resetToRoot() {
    query = "";
    results = [];
    stack = [];
    viewOwner = null;
    // Deliberately not abandoning: `hide_for_work` resets through here too, on
    // behalf of a command that is still wanted and whose next view is what
    // brings the launcher back.
    shownInvocation = null;
    panelOpen = false;
    protocolError = false;
    workingTitle = null;
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
      shownInvocation = null;
    } else if (response.kind === "removed") {
      runSearch(query);
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
    } else if (response.kind === "replaced" || response.kind === "removed") {
      stack = [...stack.slice(0, -1), response.tree];
      shownInvocation = null;
    } else if (response.kind === "failed") {
      failure = response.message;
    }
    // "started" leaves the view where it is; what it started arrives on the
    // render channel and pushes its own.
  }

  /// Confirming a result acts on it: a command is invoked, anything else runs
  /// its primary action. A command carries no actions, which is what tells the
  /// two apart.
  async function confirm(item: ResultItem | undefined) {
    if (!item) return;
    if (item.actions.length === 0) {
      failure = null;
      panelOpen = false;
      workingTitle = item.title;
      // The view that follows carries its own owner, so nothing is read back
      // here; only a refusal to start the command needs handling.
      await invoke("invoke_command", { commandId: item.id }).catch((reason) => {
        failure = String(reason);
        workingTitle = null;
      });
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
      failure = null;
      if (stack.length > 0) {
        // A command may still be working behind this view, and the user has
        // left it: stop it rather than paying for an answer nobody will read.
        invoke("cancel_invocation");
        abandonShownView();
        stack = stack.slice(0, -1);
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
        workingTitle = null;
      }),
      listen<ResultsPayload>("dango://results", (event) => {
        if (event.payload.query !== liveQuery) return;
        results = event.payload.items;
      }),
      listen<RenderPayload>("dango://render", (event) => {
        if (event.payload.invocation <= abandonedInvocation) return;
        lastInvocation = event.payload.invocation;
        workingTitle = null;
        if (event.payload.tree.protocolVersion !== PROTOCOL_VERSION) {
          protocolError = true;
          return;
        }
        viewOwner = event.payload.owner;
        if (shownInvocation === event.payload.invocation && stack.length > 0) {
          stack = [...stack.slice(0, -1), event.payload.tree];
        } else {
          stack = [...stack, event.payload.tree];
        }
        shownInvocation = event.payload.invocation;
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
  /// The footer should say what Enter does here rather than always "Select".
  /// A detail or form view declares its own actions; a list's live on its
  /// items, and every list so far gives all its items the same set.
  function stackPrimaryLabel(): string {
    const view = stack[stack.length - 1]?.view;
    if (!view) return "Select";
    const actions = view.kind === "list" ? (view.items[0]?.actions ?? []) : view.actions;
    return actions[0]?.title ?? "Select";
  }

  /// A template field needs plain Enter for a newline, so the footer has to
  /// advertise the chord that actually submits.
  function stackConfirmKey(): string {
    const view = stack[stack.length - 1]?.view;
    const hasTemplate =
      view?.kind === "form" && view.fields.some((field) => field.kind === "template");
    return hasTemplate ? `${modKey}↵` : "↵";
  }
</script>

<svelte:window onkeydown={onKeydown} onblur={() => invoke("dismiss")} />

{#snippet footer(primaryLabel: string, confirmKey: string = "↵")}
  <div
    class="border-border-card text-muted-foreground flex h-10 shrink-0 items-center justify-between border-t px-4 text-xs"
  >
    <span class="text-foreground-alt font-medium">Dango</span>
    <div class="flex items-center gap-4">
      <span class="flex items-center gap-1.5">
        {primaryLabel}
        <kbd class="bg-muted rounded px-1.5 py-0.5 font-sans">{confirmKey}</kbd>
      </span>
      <span class="flex items-center gap-1.5">
        Actions
        <kbd class="bg-muted rounded px-1.5 py-0.5 font-sans">{modKey} K</kbd>
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
    <!-- Fills the space so the footer sits on the bottom edge rather than
         directly under a short form. -->
    <div class="flex min-h-0 flex-1 flex-col">
      <ProtocolView tree={stack[stack.length - 1]} onaction={runViewAction} />
    </div>
    {#if failure}
      <div
        class="text-destructive border-border-card flex shrink-0 items-center gap-2 border-t px-5 py-2 text-sm"
      >
        <Icon name="circle-alert" size={16} />
        {failure}
      </div>
    {/if}
    {@render footer(stackPrimaryLabel(), stackConfirmKey())}
  </main>
{:else}
  <Command.Root
    shouldFilter={false}
    disablePointerSelection
    vimBindings={false}
    bind:value={() => selectedId, (id) => inUserGesture() && (pickedId = id)}
    class="border-border-card bg-background flex h-screen w-screen flex-col overflow-hidden rounded-[14px] border"
  >
    <Command.Input
      bind:ref={inputEl}
      bind:value={() => query, (q) => ((query = q), (pickedId = ""), (failure = null))}
      placeholder="Search apps and commands"
      spellcheck={false}
      autocomplete="off"
      class="text-foreground placeholder:text-muted-foreground h-16 w-full shrink-0 bg-transparent px-5 text-2xl focus:outline-none"
    />
    <!-- The inset lives outside the scroller, so the gap above the first row
         and below the last one is there at every scroll position instead of
         appearing only at the two ends. -->
    <div class="border-border-card flex min-h-0 flex-1 flex-col border-t py-2">
      <Command.List
        bind:ref={listEl}
        data-pointer={pointerActive() ? "" : undefined}
        class="min-h-0 flex-1 overflow-y-auto"
      >
        <Command.Viewport class="px-2">
          {#each results as item (item.id)}
            <!-- The row is not focusable, so a click would otherwise move focus
                 off the prompt and leave the launcher unable to type. -->
            <Command.Item
              value={item.id}
              onSelect={() => confirm(item)}
              onmousedown={(event) => event.preventDefault()}
              class="data-[selected]:bg-muted [[data-pointer]_&:hover:not([data-selected])]:bg-muted/50 flex h-14 items-center gap-3 rounded-lg px-3"
            >
              <ResultRow
                title={item.title}
                subtitle={item.subtitle}
                icon={item.icon}
                matchPositions={item.matchPositions}
                tint={item.tint}
              />
            </Command.Item>
          {/each}
          {#if results.length === 0 && query.length > 0}
            <div class="text-muted-foreground truncate px-3 py-4 text-sm">
              No results for “{query}”
            </div>
          {/if}
        </Command.Viewport>
      </Command.List>
    </div>

    {#if workingTitle}
      <div
        class="text-muted-foreground border-border-card flex shrink-0 items-center gap-2 border-t px-5 py-2 text-sm"
      >
        <Icon name="loader-circle" size={16} class="animate-spin" />
        <span class="truncate">Running {workingTitle}…</span>
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
