<script lang="ts">
  import type { ActionDto } from "./types";

  interface Props {
    actions: ActionDto[];
    onrun: (actionId: string) => void;
    onclose: () => void;
  }

  let { actions, onrun, onclose }: Props = $props();
  let selected = $state(0);

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selected = Math.min(selected + 1, actions.length - 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selected = Math.max(selected - 1, 0);
    } else if (event.key === "Enter") {
      event.preventDefault();
      onrun(actions[selected].id);
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onclose();
    }
  }

  function label(action: ActionDto): string {
    if (!action.shortcut) return "";
    const mods = action.shortcut.modifiers
      .map((m) => (m === "cmd" || m === "ctrl" ? "Ctrl" : m[0].toUpperCase() + m.slice(1)))
      .join("+");
    return mods ? `${mods}+${action.shortcut.key.toUpperCase()}` : action.shortcut.key.toUpperCase();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div
  class="border-border-card bg-background/95 absolute bottom-2 right-2 w-72 overflow-hidden rounded-[10px] border shadow-xl backdrop-blur-xl"
>
  {#each actions as action, i (action.id)}
    <button
      type="button"
      class="flex w-full items-center justify-between px-3 py-2 text-left text-sm {i === selected
        ? 'bg-muted text-foreground'
        : 'text-foreground-alt'}"
      onmouseenter={() => (selected = i)}
      onclick={() => onrun(action.id)}
    >
      <span>{action.title}</span>
      {#if action.shortcut}
        <span class="text-muted-foreground text-xs">{label(action)}</span>
      {/if}
    </button>
  {/each}
</div>
