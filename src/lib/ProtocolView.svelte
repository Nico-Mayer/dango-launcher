<script lang="ts">
  import { Command } from "bits-ui";
  import ActionPanel from "./ActionPanel.svelte";
  import FormFields from "./FormFields.svelte";
  import ResultRow from "./ResultRow.svelte";
  import { inUserGesture, pointerActive } from "./input.svelte";
  import type { ViewTree } from "../protocol/ViewTree";
  import { matchesShortcut, type ActionDto } from "./types";

  interface Props {
    tree: ViewTree;
    onaction: (actionId: string, itemId: string | null, values?: Record<string, string>) => void;
  }

  let { tree, onaction }: Props = $props();

  let query = $state("");
  let pickedId = $state("");
  let panelOpen = $state(false);
  let inputEl = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLElement | null>(null);

  const view = $derived(tree.view);

  /// A form submit is an action on the form carrying the field values, which is
  /// the same path every other action takes. The fields own their own state, so
  /// this only forwards.
  function submitForm(values: Record<string, string>) {
    const action = primaryAction();
    if (action) onaction(action, view.kind === "form" ? view.itemId : null, values);
  }

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

  /// A list that has rows always has one of them selected, whatever narrowing
  /// or a new tree did to the items.
  const selectedId = $derived(
    items.some((item) => item.id === pickedId) ? pickedId : (items[0]?.id ?? ""),
  );
  const selectedItem = $derived(items.find((item) => item.id === selectedId));

  /// Same first-row blind spot in the primitive's scroll-into-view as the root
  /// list has: without a group heading to scroll to, it scrolls nothing.
  $effect(() => {
    if (selectedId && selectedId === items[0]?.id && listEl) listEl.scrollTop = 0;
  });

  /// What the action panel offers. A list item's own actions win over the
  /// view's, because the item is what the user has selected.
  const actions = $derived<ActionDto[]>(
    view.kind === "list" ? (selectedItem?.actions ?? []) : view.actions,
  );

  /// Which item the action applies to. Lists use their selection, forms carry
  /// the record that opened them, and detail views have none.
  function selectedItemId(): string | null {
    if (view.kind === "list") return selectedItem?.id ?? null;
    if (view.kind === "form") return view.itemId;
    return null;
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
    // A form's Enter belongs to its own fields, where a template field has to
    // be able to make a newline.
    if (view.kind === "form") return;
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
    disablePointerSelection
    vimBindings={false}
    bind:value={() => selectedId, (id) => inUserGesture() && (pickedId = id)}
    class="flex min-h-0 flex-1 flex-col"
  >
    <Command.Input
      bind:ref={inputEl}
      bind:value={() => query, (q) => ((query = q), (pickedId = ""))}
      placeholder="Search"
      spellcheck={false}
      autocomplete="off"
      class="text-foreground placeholder:text-muted-foreground h-16 w-full shrink-0 bg-transparent px-5 text-2xl focus:outline-none"
    />
    <div class="border-border-card flex min-h-0 flex-1 flex-col border-t py-2">
      <Command.List
        bind:ref={listEl}
        data-pointer={pointerActive() ? "" : undefined}
        class="min-h-0 flex-1 overflow-y-auto"
      >
        <Command.Viewport class="px-2">
          {#each items as item (item.id)}
            <Command.Item
              value={item.id}
              onSelect={() => item.actions.length > 0 && onaction(item.actions[0].id, item.id)}
              onmousedown={(event) => event.preventDefault()}
              class="data-[selected]:bg-muted [[data-pointer]_&:hover:not([data-selected])]:bg-muted/50 flex h-14 items-center gap-3 rounded-lg px-3"
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
                Loading…
              {:else if query.length > 0}
                <span class="block truncate">No results for “{query}”</span>
              {:else}
                {view.emptyState?.title ?? "Nothing to show"}
                {#if view.emptyState?.description}
                  <p class="mt-1 text-xs">{view.emptyState.description}</p>
                {/if}
              {/if}
            </div>
          {/if}
        </Command.Viewport>
      </Command.List>
    </div>
  </Command.Root>
{:else if view.kind === "detail"}
  <div class="text-foreground min-h-0 flex-1 overflow-y-auto px-5 py-4 text-sm whitespace-pre-wrap">
    {view.markdown}
  </div>
{:else if view.kind === "form"}
  {#key tree}
    <FormFields {view} onsubmit={submitForm} />
  {/key}
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
