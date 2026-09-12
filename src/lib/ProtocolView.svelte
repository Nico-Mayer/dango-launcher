<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import ActionPanel from "./ActionPanel.svelte";
  import Icon, { namedIcon } from "./Icon.svelte";
  import { pointerOwnsSelection } from "./pointer.svelte";
  import type { ViewTree } from "../protocol/ViewTree";
  import { matchesShortcut, type ActionDto } from "./types";

  interface Props {
    tree: ViewTree;
    /// What the user has typed. Applied here only when the view declares that
    /// the launcher owns filtering.
    query: string;
    onaction: (actionId: string, itemId: string | null) => void;
    onsubmit: (values: Record<string, string>) => void;
  }

  let { tree, query, onaction, onsubmit }: Props = $props();
  let selected = $state(0);
  let panelOpen = $state(false);
  let listEl = $state<HTMLUListElement | null>(null);
  let formValues = $state<Record<string, string>>({});

  const view = $derived(tree.view);

  const items = $derived(
    view.kind !== "list"
      ? []
      : view.filtering === "launcher" && query.length > 0
        ? view.items.filter((item) => item.title.toLowerCase().includes(query.toLowerCase()))
        : view.items,
  );

  // Narrowing the list can strand the cursor past its end.
  $effect(() => {
    if (selected >= items.length) selected = Math.max(items.length - 1, 0);
  });

  // Arrowing past the visible edge has to bring the selection with it. The
  // root list gets this from its primitive; a pushed view has none.
  $effect(() => {
    const index = selected;
    listEl?.children[index]?.scrollIntoView({ block: "nearest" });
  });

  /// What the action panel offers. A list item's own actions win over the
  /// view's, because the item is what the user has selected.
  const actions = $derived<ActionDto[]>(
    view.kind === "list" ? (items[selected]?.actions ?? []) : view.actions,
  );

  /// Which item the action applies to. A detail or form view has none, and the
  /// command is expected to already know what it asked about.
  function selectedItemId(): string | null {
    return view.kind === "list" ? (items[selected]?.id ?? null) : null;
  }

  function primaryAction(): string | null {
    const actions = "actions" in view ? view.actions : [];
    return actions.length > 0 ? actions[0].id : null;
  }

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
    if (view.kind === "list") {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        selected = Math.min(selected + 1, items.length - 1);
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        selected = Math.max(selected - 1, 0);
      }
    }
    if (event.key === "Enter") {
      if (view.kind === "form") {
        event.preventDefault();
        onsubmit(formValues);
        return;
      }
      const action =
        view.kind === "list" && items[selected]?.actions.length
          ? items[selected].actions[0].id
          : primaryAction();
      if (action) {
        event.preventDefault();
        onaction(action, selectedItemId());
      }
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />


{#if view.kind === "list"}
  {#if view.loading && items.length === 0}
    <div class="text-muted-foreground px-4 py-6 text-sm">Loading...</div>
  {:else if items.length === 0}
    <div class="text-muted-foreground px-4 py-6 text-sm">
      {query.length > 0 ? "No results" : (view.emptyState?.title ?? "Nothing here")}
    </div>
  {:else}
    <ul bind:this={listEl} class="py-1">
      {#each items as item, i (item.id)}
        <li>
          <button
            type="button"
            class="mx-2 flex w-[calc(100%-1rem)] items-center gap-3 rounded-lg px-3 py-2 text-left {i ===
            selected
              ? 'bg-muted'
              : ''}"
            onpointermove={() => pointerOwnsSelection() && (selected = i)}
            onclick={() => item.actions.length > 0 && onaction(item.actions[0].id, item.id)}
          >
            {#if namedIcon(item.icon)}
              <div class="text-foreground-alt flex h-10 w-10 shrink-0 items-center justify-center">
                <Icon name={namedIcon(item.icon)!} size={18} />
              </div>
            {:else if item.icon}
              <!-- Big enough to tell one copied image from another, which a
                   row-height thumbnail is not. -->
              <img
                src={convertFileSrc(item.icon)}
                alt=""
                class="border-border-card h-10 w-10 shrink-0 rounded border object-cover"
              />
            {:else}
              <div class="bg-muted h-10 w-10 shrink-0 rounded"></div>
            {/if}
            <div class="flex min-w-0 flex-col">
              <span class="text-foreground truncate text-sm">{item.title}</span>
              {#if item.subtitle}
                <span class="text-muted-foreground truncate text-xs">{item.subtitle}</span>
              {/if}
            </div>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
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
