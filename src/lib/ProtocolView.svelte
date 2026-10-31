<script lang="ts">
  import * as Command from "./ui/command";
  import type { Snippet } from "svelte";
  import LauncherFooter from "./launcher/LauncherFooter.svelte";
  import { modKey } from "./platform";
  import ActionPanel from "./ActionPanel.svelte";
  import FormFields from "./FormFields.svelte";
  import EmptyMessage from "./ui/EmptyMessage.svelte";
  import ResultRow from "./ResultRow.svelte";
  import { pointerActive } from "./input.svelte";
  import { dispatch, echoed } from "./launcher.svelte";
  import type { Snapshot } from "../protocol/Snapshot";
  import type { ViewScreen } from "../protocol/ViewScreen";

  interface Props {
    snapshot: Snapshot;
    screen: ViewScreen;
    beforeFooter?: Snippet;
  }

  let { snapshot, screen, beforeFooter }: Props = $props();

  let panelPresent = $state(false);
  let contentEl = $state<HTMLDivElement | null>(null);

  const body = $derived(screen.body);
  const footer = $derived(snapshot.footer);
  const query = echoed(() => (screen.body.kind === "list" ? screen.body.query : { text: "", revision: -1 }));

  /// Where the cursor goes back to when the action panel closes: whatever in
  /// the view had it, or the view's own surface.
  function returnFocus() {
    const focused = document.activeElement;
    return focused instanceof HTMLElement && contentEl?.contains(focused)
      ? focused
      : contentEl?.querySelector<HTMLElement>("[data-surface], input, textarea, button") ?? null;
  }
</script>

<div bind:this={contentEl} aria-busy={screen.busy} class="flex min-h-0 flex-1 flex-col">
{#if body.kind === "list"}
  <Command.Root bind:value={() => body.selected ?? "", () => {}}>
    <Command.Input
      data-surface="viewInput"
      bind:value={() => query.text, (text) => {
        query.text = text;
        dispatch({ kind: "queryChanged", text });
      }}
      placeholder="Search"
    />
    <div class="border-border-card flex min-h-0 flex-1 flex-col border-t-edge py-2">
      <Command.List data-pointer={pointerActive() ? "" : undefined}>
        <Command.Viewport>
          {#each body.items as item (item.id)}
            <Command.Item
              value={item.id}
              onSelect={() => !panelPresent && dispatch({ kind: "activate", id: item.id })}
              onmousedown={(event) => event.preventDefault()}
            >
              <ResultRow
                title={item.title}
                subtitle={item.subtitle}
                icon={item.icon}
                iconIsContent
              />
            </Command.Item>
          {/each}
          {#if body.empty}
            <EmptyMessage>
              {#if body.empty.truncate}
                <span class="block truncate">{body.empty.title}</span>
              {:else}
                {body.empty.title}
                {#if body.empty.description}
                  <p class="mt-1 text-xs">{body.empty.description}</p>
                {/if}
              {/if}
            </EmptyMessage>
          {/if}
        </Command.Viewport>
      </Command.List>
    </div>
  </Command.Root>
{:else if body.kind === "detail"}
  <div data-surface="detail" tabindex="-1" class="text-foreground min-h-0 flex-1 overflow-y-auto px-5 py-4 text-sm whitespace-pre-wrap wrap-anywhere outline-none">
    {body.markdown}
  </div>
{:else}
  {#key body.revision}
    <FormFields form={body} />
  {/key}
{/if}
</div>
{@render beforeFooter?.()}
<LauncherFooter
  status={footer.status}
  hideStatus={footer.hideStatus}
  primaryLabel={footer.primaryLabel}
  confirmKey={footer.confirmWithCommand ? `${modKey}↵` : "↵"}
>
  {#snippet actions()}
    <ActionPanel panel={snapshot.panel} bind:present={panelPresent} {returnFocus} />
  {/snippet}
</LauncherFooter>
