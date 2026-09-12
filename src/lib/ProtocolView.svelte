<script lang="ts">
  import { Command } from "bits-ui";
  import ActionPanel from "./ActionPanel.svelte";
  import ResultRow from "./ResultRow.svelte";
  import { pointerOwnsSelection } from "./pointer.svelte";
  import type { ViewTree } from "../protocol/ViewTree";
  import { matchesShortcut, type ActionDto } from "./types";

  interface Props {
    tree: ViewTree;
    onaction: (actionId: string, itemId: string | null) => void;
    onsubmit: (values: Record<string, string>) => void;
  }

  let { tree, onaction, onsubmit }: Props = $props();

  let query = $state("");
  let selectedId = $state("");
  let panelOpen = $state(false);
  let inputEl = $state<HTMLInputElement | null>(null);
  let formValues = $state<Record<string, string>>({});

  const view = $derived(tree.view);

  /// Narrowing happens here only when the view says the launcher owns it. A
  /// view that owns its own filtering gets each query change instead, which no
  /// command needs yet.
  const items = $derived(
    view.kind !== "list"
      ? []
      : view.filtering === "launcher" && query.length > 0
        ? view.items.filter((item) => item.title.toLowerCase().includes(query.toLowerCase()))
        : view.items,
  );

  const selectedItem = $derived(items.find((item) => item.id === selectedId) ?? items[0]);

  /// What the action panel offers. A list item's own actions win over the
  /// view's, because the item is what the user has selected.
  const actions = $derived<ActionDto[]>(
    view.kind === "list" ? (selectedItem?.actions ?? []) : view.actions,
  );

  /// Which item the action applies to. A detail or form view has none, and the
  /// command is expected to already know what it asked about.
  function selectedItemId(): string | null {
    return view.kind === "list" ? (selectedItem?.id ?? null) : null;
  }

  function primaryAction(): string | null {
    const available = "actions" in view ? view.actions : [];
    return available.length > 0 ? available[0].id : null;
  }

  /// A view arriving takes the cursor, so typing narrows it without a click,
  /// and the action panel closing hands it back.
  $effect(() => {
    if (panelOpen) return;
    inputEl?.focus();
  });

  /// Keys the Command primitive does not own. Arrow navigation, Enter on a list
  /// item, and scroll-into-view all come from it, so only the action panel,
  /// per-action shortcuts, and submitting a form are handled here.
  function onKeydown(event: KeyboardEvent) {
    if (panelOpen) return;
    if (event.key.toLowerCase() === "k" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      if (actions.length > 0) panelOpen = true;
      return;
    }
    for (const action of actions) {
      if (action.shortcut && matchesShortcut(event, action.shortcut)) {
        event.preventDefault();
        onaction(action.id, selectedItemId());
        return;
      }
    }
    if (event.key !== "Enter") return;
    if (view.kind === "form") {
      event.preventDefault();
      onsubmit(formValues);
      return;
    }
    // A list's Enter belongs to the primitive, which calls onSelect.
    if (view.kind === "list") return;
    const action = primaryAction();
    if (action) {
      event.preventDefault();
      onaction(action, null);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />


{#if view.kind === "list"}
  <Command.Root
    shouldFilter={false}
    disablePointerSelection={!pointerOwnsSelection()}
    bind:value={selectedId}
    class="flex min-h-0 flex-1 flex-col"
  >
    <Command.Input
      bind:ref={inputEl}
      bind:value={query}
      placeholder="Search..."
      spellcheck={false}
      autocomplete="off"
      class="text-foreground placeholder:text-muted-foreground h-16 w-full shrink-0 bg-transparent px-5 text-2xl focus:outline-none"
    />
    <Command.List class="border-border-card min-h-0 flex-1 overflow-y-auto border-t">
      <Command.Viewport class="p-2">
        {#each items as item (item.id)}
          <Command.Item
            value={item.id}
            onSelect={() => item.actions.length > 0 && onaction(item.actions[0].id, item.id)}
            class="data-[selected]:bg-muted flex h-14 items-center gap-3 rounded-lg px-3"
          >
            <ResultRow
              title={item.title}
              subtitle={item.subtitle}
              icon={item.icon}
              iconIsContent
            />
          </Command.Item>
        {/each}
        {#if items.length === 0}
          <div class="text-muted-foreground px-3 py-4 text-sm">
            {#if view.loading}
              Loading...
            {:else if query.length > 0}
              No results
            {:else}
              {view.emptyState?.title ?? "Nothing here"}
              {#if view.emptyState?.description}
                <p class="mt-1 text-xs">{view.emptyState.description}</p>
              {/if}
            {/if}
          </div>
        {/if}
      </Command.Viewport>
    </Command.List>
  </Command.Root>
{:else if view.kind === "detail"}
  <div class="text-foreground whitespace-pre-wrap px-4 py-3 text-sm">
    {view.markdown}
  </div>
{:else if view.kind === "form"}
  <form
    class="flex flex-col gap-3 px-4 py-3"
    onsubmit={(e) => {
      e.preventDefault();
      onsubmit(formValues);
    }}
  >
    {#each view.fields as field (field.id)}
      <label class="flex flex-col gap-1">
        <span class="text-foreground-alt text-xs">{field.label}</span>
        {#if field.kind === "toggle"}
          <input
            type="checkbox"
            onchange={(e) => (formValues[field.id] = e.currentTarget.checked ? "true" : "false")}
          />
        {:else}
          <input
            type={field.kind === "password" ? "password" : "text"}
            class="border-border-input bg-background/60 text-foreground rounded-md border px-2 py-1 text-sm focus:outline-none"
            value={field.value ?? ""}
            oninput={(e) => (formValues[field.id] = e.currentTarget.value)}
          />
        {/if}
      </label>
    {/each}
  </form>
{/if}

{#if panelOpen}
  <ActionPanel
    {actions}
    onrun={(id) => {
      panelOpen = false;
      onaction(id, selectedItemId());
    }}
    onclose={() => (panelOpen = false)}
  />
{/if}
