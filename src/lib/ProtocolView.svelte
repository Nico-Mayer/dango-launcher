<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { ViewTree } from "../protocol/ViewTree";

  interface Props {
    tree: ViewTree;
    onaction: (actionId: string, itemId: string | null) => void;
    onsubmit: (values: Record<string, string>) => void;
  }

  let { tree, onaction, onsubmit }: Props = $props();
  let selected = $state(0);
  let formValues = $state<Record<string, string>>({});

  const view = $derived(tree.view);

  /// Which item the action applies to. A detail or form view has none, and the
  /// command is expected to already know what it asked about.
  function selectedItemId(): string | null {
    return view.kind === "list" ? (view.items[selected]?.id ?? null) : null;
  }

  function primaryAction(): string | null {
    const actions = "actions" in view ? view.actions : [];
    return actions.length > 0 ? actions[0].id : null;
  }

  function onKeydown(event: KeyboardEvent) {
    if (view.kind === "list") {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        selected = Math.min(selected + 1, view.items.length - 1);
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
        view.kind === "list" && view.items[selected]?.actions.length
          ? view.items[selected].actions[0].id
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
  {#if view.loading && view.items.length === 0}
    <div class="text-muted-foreground px-4 py-6 text-sm">Loading...</div>
  {:else if view.items.length === 0}
    <div class="text-muted-foreground px-4 py-6 text-sm">
      {view.emptyState?.title ?? "Nothing here"}
    </div>
  {:else}
    <ul class="max-h-[420px] overflow-y-auto py-1">
      {#each view.items as item, i (item.id)}
        <li>
          <button
            type="button"
            class="mx-2 flex w-[calc(100%-1rem)] items-center gap-3 rounded-lg px-3 py-2 text-left {i ===
            selected
              ? 'bg-muted'
              : ''}"
            onmouseenter={() => (selected = i)}
            onclick={() => item.actions.length > 0 && onaction(item.actions[0].id, item.id)}
          >
            {#if item.icon}
              <img src={convertFileSrc(item.icon)} alt="" class="h-6 w-6 shrink-0" />
            {:else}
              <div class="bg-muted h-6 w-6 shrink-0 rounded"></div>
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
  <div class="text-foreground max-h-[420px] overflow-y-auto whitespace-pre-wrap px-4 py-3 text-sm">
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
