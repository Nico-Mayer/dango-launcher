<script lang="ts">
  import { Command, Popover } from "bits-ui";
  import { onMount } from "svelte";
  import Shortcut from "./ui/Shortcut.svelte";
  import { pointerActive } from "./input.svelte";
  import { dispatch } from "./launcher.svelte";
  import { modKey, shortcutLabel } from "./platform";
  import type { Panel } from "../protocol/Panel";

  let { panel, present = $bindable(false), returnFocus }: {
    panel: Panel;
    present?: boolean;
    returnFocus: () => HTMLElement | null;
  } = $props();

  const uid = $props.id();
  const open = $derived(panel.open);
  let commandEl = $state<HTMLDivElement | null>(null);
  let contentEl = $state<HTMLDivElement | null>(null);
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let origin: HTMLElement | null = null;
  let closingKey: string | null = null;
  const optionId = (id: string) => `${uid}-action-${encodeURIComponent(id)}`;

  $effect(() => {
    if (panel.selected) document.getElementById(optionId(panel.selected))?.scrollIntoView({ block: "nearest" });
  });

  function changeOpen(value: boolean) {
    if (value === open) return;
    if (value) {
      origin = returnFocus();
      present = true;
      dispatch({ kind: "togglePanel" });
    } else {
      dispatch({ kind: "closePanel" });
    }
  }

  // The exit guard: input crossing the closing boundary must not reach the
  // parent while the panel is still on screen, nor may a held closing key.
  onMount(() => {
    function keydown(event: KeyboardEvent) {
      if ((!open && present) || (event.repeat && event.key === closingKey)) {
        event.preventDefault();
        event.stopImmediatePropagation();
        return;
      }
      if (open && (event.key === "Enter" || event.key === "Escape")) closingKey = event.key;
    }
    function keyup(event: KeyboardEvent) {
      if (event.key === closingKey) closingKey = null;
    }
    let outsidePress = false;
    function pointerdown(event: PointerEvent) {
      const target = event.target;
      outsidePress = (open || present) && target instanceof Node
        && !contentEl?.contains(target) && !triggerEl?.contains(target);
    }
    // Bits dismisses after pointerdown; consume the click even if exit already finished.
    function mousedown(event: MouseEvent) {
      if (outsidePress) event.preventDefault();
    }
    function click(event: MouseEvent) {
      if (!outsidePress) return;
      outsidePress = false;
      event.preventDefault();
      event.stopImmediatePropagation();
    }
    window.addEventListener("pointerdown", pointerdown, true);
    window.addEventListener("mousedown", mousedown, true);
    window.addEventListener("click", click, true);
    window.addEventListener("keydown", keydown, true);
    window.addEventListener("keyup", keyup, true);
    return () => {
      window.removeEventListener("pointerdown", pointerdown, true);
      window.removeEventListener("mousedown", mousedown, true);
      window.removeEventListener("click", click, true);
      window.removeEventListener("keydown", keydown, true);
      window.removeEventListener("keyup", keyup, true);
    };
  });
</script>

<Popover.Root bind:open={() => open, changeOpen} onOpenChangeComplete={(value) => { if (!value) present = false; }}>
  <Popover.Trigger
    bind:ref={triggerEl}
    data-native-keys
    disabled={panel.actions.length === 0}
    aria-label="Actions"
    class="dango-button flex shrink-0 items-center gap-1.5 rounded-button text-xs focus-visible:outline-control-focus disabled:text-text-disabled"
    onmousedown={(event) => event.preventDefault()}
  >
    Actions <Shortcut>{modKey} K</Shortcut>
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content
      bind:ref={contentEl}
      class="action-panel border-border-card bg-background-alt z-overlay overflow-hidden rounded-overlay border-edge shadow-xl"
      side="top" align="end" sideOffset={8} collisionPadding={8}
      onOpenAutoFocus={(event) => {
        event.preventDefault();
        origin ??= returnFocus();
        present = true;
        commandEl?.focus();
      }}
      onCloseAutoFocus={(event) => {
        event.preventDefault();
        (origin?.isConnected ? origin : returnFocus())?.focus();
        origin = null;
      }}
    >
      <Command.Root
        data-pointer={pointerActive() ? "" : undefined}
        bind:ref={commandEl} bind:value={() => panel.selected ?? "", () => {}}
        label="Actions" aria-label="Actions"
        aria-activedescendant={panel.selected ? optionId(panel.selected) : undefined}
        aria-controls={`${uid}-list`}
        shouldFilter={false} disablePointerSelection vimBindings={false} loop={false}
        class="outline-none"
      >
        <Command.List id={`${uid}-list`} aria-label="Actions" class="action-list overflow-y-auto">
          <Command.Viewport>
            {#each panel.actions as action (action.id)}
              <Command.Item
                id={optionId(action.id)} value={action.id} disabled={!open}
                onSelect={() => open && dispatch({ kind: "activate", id: action.id })}
                onmousedown={(event) => event.preventDefault()}
                class="flex cursor-default items-center justify-between gap-3 px-3 py-2 text-sm text-foreground-alt data-selected:bg-selection-bg data-selected:text-selection-text data-selected:inset-shadow-selection [&:not([data-selected]):hover]:[[data-pointer]_&]:bg-hover-bg"
              >
                <span class="min-w-0 truncate">{action.title}</span>
                {#if action.shortcut}
                  <span class="shrink-0 text-muted-foreground text-xs">{shortcutLabel(action.shortcut.key, action.shortcut.modifiers)}</span>
                {/if}
              </Command.Item>
            {/each}
          </Command.Viewport>
        </Command.List>
      </Command.Root>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global(.action-panel) {
    width: min(var(--dango-overlay-width), var(--bits-popover-content-available-width));
    max-height: var(--bits-popover-content-available-height);
  }
  :global(.action-list) {
    max-height: calc(var(--bits-popover-content-available-height) - 2 * var(--dango-edge-width));
  }
</style>
