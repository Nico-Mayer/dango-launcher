<script lang="ts">
  import { onMount } from "svelte";
  import { pointerActive } from "./input.svelte";
  import type { ActionDto } from "./types";

  interface Props {
    actions: ActionDto[];
    onrun: (actionId: string) => void;
    onclose: () => void;
  }

  let { actions, onrun, onclose }: Props = $props();
  let selected = $state(0);

  // Capture phase so the panel owns these keys while open, before the Command
  // list beneath it can act on them.
  onMount(() => {
    function onKeydown(event: KeyboardEvent) {
      if (event.key === "ArrowDown") {
        selected = Math.min(selected + 1, actions.length - 1);
      } else if (event.key === "ArrowUp") {
        selected = Math.max(selected - 1, 0);
      } else if (event.key === "Enter") {
        onrun(actions[selected].id);
      } else if (event.key === "Escape") {
        onclose();
      } else {
        return;
      }
      event.preventDefault();
      event.stopImmediatePropagation();
    }
    window.addEventListener("keydown", onKeydown, true);
    return () => window.removeEventListener("keydown", onKeydown, true);
  });

  function label(action: ActionDto): string {
    if (!action.shortcut) return "";
    const mods = action.shortcut.modifiers
      .map((m) => (m === "cmd" || m === "ctrl" ? "Ctrl" : m[0].toUpperCase() + m.slice(1)))
      .join("+");
    return mods ? `${mods}+${action.shortcut.key.toUpperCase()}` : action.shortcut.key.toUpperCase();
  }
</script>

<!-- mousedown is prevented on each entry: these are real buttons, and letting
     one take focus would strand the cursor outside the search input once the
     panel closes. -->
<div
  data-pointer={pointerActive() ? "" : undefined}
  class="border-border-card bg-background-alt absolute bottom-12 right-2 w-72 overflow-hidden rounded-[10px] border shadow-xl"
>
  {#each actions as action, i (action.id)}
    <button
      type="button"
      class="flex w-full items-center justify-between px-3 py-2 text-left text-sm {i === selected
        ? 'bg-muted text-foreground'
        : 'text-foreground-alt [[data-pointer]_&:hover]:bg-muted/50'}"
      onmousedown={(event) => event.preventDefault()}
      onclick={() => onrun(action.id)}
    >
      <span>{action.title}</span>
      {#if action.shortcut}
        <span class="text-muted-foreground text-xs">{label(action)}</span>
      {/if}
    </button>
  {/each}
</div>
